<#
    Puts the process that runs a build step into a job the system takes down with it.

    A step is over when its script ends, but a process the script leaves behind
    keeps running on the build agent for as long as the agent lives: a wizard a
    failing case did not close, the toolchain's telemetry helper the linker starts.
    Everything the script starts is therefore put in a job of the script's own.

    A process that joins a job hands that job to every process it starts
    afterwards, and a job created here is destroyed with the last handle to it,
    which is the one the joining script holds. Joining it therefore means that
    when the step's script exits, whatever it left behind goes with it.

    Joining is best effort: a host may already run its steps in a job of its own
    that refuses a second one, and a step that cannot join still runs -- it only
    leaves what it started running on the agent. Either way the step says what
    happened, so a step that goes wrong still reports whether the guard was in
    place. What keeps a step from ending is a different matter, and it is settled
    where the command is started: a child of the step inherits the step's own
    output, so a command is run with a console of its own.
#>

Set-StrictMode -Version Latest

if (-not ("NanoStepJob" -as [type])) {
    Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public static class NanoStepJob
{
    [StructLayout(LayoutKind.Sequential)]
    private struct BasicLimitInformation
    {
        public long PerProcessUserTimeLimit;
        public long PerJobUserTimeLimit;
        public uint LimitFlags;
        public UIntPtr MinimumWorkingSetSize;
        public UIntPtr MaximumWorkingSetSize;
        public uint ActiveProcessLimit;
        public UIntPtr Affinity;
        public uint PriorityClass;
        public uint SchedulingClass;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct IoCounters
    {
        public ulong ReadOperationCount;
        public ulong WriteOperationCount;
        public ulong OtherOperationCount;
        public ulong ReadTransferCount;
        public ulong WriteTransferCount;
        public ulong OtherTransferCount;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct ExtendedLimitInformation
    {
        public BasicLimitInformation BasicLimitInformation;
        public IoCounters IoInfo;
        public UIntPtr ProcessMemoryLimit;
        public UIntPtr JobMemoryLimit;
        public UIntPtr PeakProcessMemoryUsed;
        public UIntPtr PeakJobMemoryUsed;
    }

    private const int ExtendedLimitInformationClass = 9;
    private const uint KillOnJobClose = 0x2000;

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateJobObject(IntPtr attributes, string name);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool SetInformationJobObject(IntPtr job, int informationClass, IntPtr information, uint length);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);

    public static IntPtr Create()
    {
        IntPtr job = CreateJobObject(IntPtr.Zero, null);
        if (job == IntPtr.Zero)
        {
            return IntPtr.Zero;
        }
        ExtendedLimitInformation limits = new ExtendedLimitInformation();
        limits.BasicLimitInformation.LimitFlags = KillOnJobClose;
        int size = Marshal.SizeOf(typeof(ExtendedLimitInformation));
        IntPtr buffer = Marshal.AllocHGlobal(size);
        try
        {
            Marshal.StructureToPtr(limits, buffer, false);
            if (!SetInformationJobObject(job, ExtendedLimitInformationClass, buffer, (uint)size))
            {
                return IntPtr.Zero;
            }
        }
        finally
        {
            Marshal.FreeHGlobal(buffer);
        }
        return job;
    }

    public static bool Join(IntPtr job, IntPtr process)
    {
        return AssignProcessToJobObject(job, process);
    }
}
"@
}

if (-not ("NanoStepCommand" -as [type])) {
    Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public static class NanoStepCommand
{
    [StructLayout(LayoutKind.Sequential)]
    private struct StartupInfo
    {
        public int Size;
        public IntPtr Reserved;
        public IntPtr Desktop;
        public IntPtr Title;
        public int X;
        public int Y;
        public int XSize;
        public int YSize;
        public int XCountChars;
        public int YCountChars;
        public int FillAttribute;
        public int Flags;
        public short ShowWindow;
        public short Reserved2;
        public IntPtr Reserved3;
        public IntPtr StdInput;
        public IntPtr StdOutput;
        public IntPtr StdError;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct ProcessInformation
    {
        public IntPtr Process;
        public IntPtr Thread;
        public int ProcessId;
        public int ThreadId;
    }

    private const uint CreateNoWindow = 0x08000000;
    private const uint StartfUseShowWindow = 0x00000001;
    private const short SwHide = 0;
    private const uint WaitTimeout = 0x00000102;

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool CreateProcessW(
        string application,
        string commandLine,
        IntPtr processAttributes,
        IntPtr threadAttributes,
        bool inheritHandles,
        uint creationFlags,
        IntPtr environment,
        string currentDirectory,
        ref StartupInfo startupInfo,
        out ProcessInformation processInformation);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern uint WaitForSingleObject(IntPtr handle, uint milliseconds);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetExitCodeProcess(IntPtr process, out uint exitCode);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern int GetProcessId(IntPtr process);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool TerminateProcess(IntPtr process, uint exitCode);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool CloseHandle(IntPtr handle);

    // Starts a command line with no console of its own and *without* handing this
    // process's handles to it.
    //
    // Both halves matter. A build agent gives its step a pipe for standard output
    // and standard error and calls the step finished when that pipe closes, so
    // every handle a descendant inherits is a chance for the step never to end:
    // `Process.Start`, whichever of its two paths is used, creates the process
    // with handle inheritance on, and one process that outlives the command then
    // keeps the step's output open for as long as it lives. Measured on this
    // machine with a grandchild living eight seconds: started the ordinary way,
    // the caller's output stayed open 7.5 s after the step's own process had
    // gone; started this way, it closed at once.
    //
    // The command is given no console at all rather than one of its own. A
    // console has to be created by the system before the process can run, and
    // that handshake is one more thing a step can stop inside; the command writes
    // its output to a file the shell opens for it, so it needs no console.
    public static IntPtr Start(string commandLine, string currentDirectory)
    {
        StartupInfo startup = new StartupInfo();
        startup.Size = Marshal.SizeOf(typeof(StartupInfo));
        startup.Flags = (int)StartfUseShowWindow;
        startup.ShowWindow = SwHide;
        ProcessInformation information;
        if (!CreateProcessW(null, commandLine, IntPtr.Zero, IntPtr.Zero, false, CreateNoWindow, IntPtr.Zero, currentDirectory, ref startup, out information))
        {
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        }
        CloseHandle(information.Thread);
        return information.Process;
    }

    public static int Id(IntPtr process)
    {
        return GetProcessId(process);
    }

    // Waits for the command and says whether it ended within the deadline; a
    // deadline of zero or less waits without one.
    public static bool Wait(IntPtr process, int milliseconds)
    {
        uint waited = WaitForSingleObject(process, milliseconds <= 0 ? 0xFFFFFFFF : (uint)milliseconds);
        if (waited == WaitTimeout)
        {
            return false;
        }
        if (waited != 0)
        {
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        }
        return true;
    }

    public static int ExitCode(IntPtr process)
    {
        uint code;
        if (!GetExitCodeProcess(process, out code))
        {
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        }
        return unchecked((int)code);
    }

    public static void Kill(IntPtr process)
    {
        TerminateProcess(process, 1);
    }

    public static void Release(IntPtr process)
    {
        CloseHandle(process);
    }
}
"@
}

# The handle is kept for the whole life of the script: the job is over once the
# last handle to it closes, and the process holding this one is the step itself.
$script:StepJobHandle = [IntPtr]::Zero

# Where the step writes what it is doing, for the watchdog below to read. One
# line at a time, appended before the thing that could block, so a step that
# stops inside that thing has already said where it was.
$script:StepCrumbPath = ""

$job = [NanoStepJob]::Create()
if ($job -eq [IntPtr]::Zero) {
    Write-Warning "step job: the system refused a job for this step, so a process it leaves behind can hold the step open"
}
elseif (-not [NanoStepJob]::Join($job, [System.Diagnostics.Process]::GetCurrentProcess().Handle)) {
    Write-Warning "step job: this host does not allow a second job here, so a process this step leaves behind can hold it open"
}
else {
    $script:StepJobHandle = $job
    Write-Output "step job: everything this step starts ends with it"
}

<#
    Says what this step is doing, where a process outside the step can read it.

    A local file append, and deliberately nothing else: the point of the record
    is that it survives a step which stops answering, and a step cannot write a
    commit status from inside a call that never returns. scripts/step_observer.ps1
    reads these lines and posts them; this side only appends, so it can never be
    the thing that blocks. A line naming a command's output file (`log <path>`)
    is how the observer learns which file to tail.
#>
function Add-StepCrumb {
    param([string]$Message)

    if (-not $script:StepCrumbPath) {
        return
    }
    try {
        [System.IO.File]::AppendAllText($script:StepCrumbPath, $Message + "`r`n", (New-Object System.Text.UTF8Encoding($false)))
    }
    catch {
        return
    }
}

<#
    Where a step is, written the way the watchdog above reads it.

    It is deliberately not a commit status written from the step. Three runs of
    this repository stopped at the first report a step made while a command was
    running and never made another: a status written from inside a step is a
    network call the step waits on, and Windows PowerShell implements
    `Invoke-RestMethod` on `HttpWebRequest`, whose `Timeout` covers getting the
    response and not reading its body -- measured here against a server that
    writes its headers and then nothing, `-TimeoutSec 10` was still waiting after
    45 s. The record of where a step stopped cannot be the reason it stopped, so
    the record is a line the step appends to a file and the watchdog posts it.
#>
function Update-PhaseStatus {
    param([string]$Message)

    $stamp = (Get-Date).ToUniversalTime().ToString("HH:mm:ss")
    Add-StepCrumb "$stamp $Message"
}

<#
    Watches this step from outside it, and ends it if the step is still there
    when its time is up.

    A step is finished when its output closes, and a step that stops answering
    never closes it: nothing on the runner's side reaches such a step, and the
    branch inside the scripts that is supposed to end a command which never ends
    is itself inside the step. This is a process of its own that ends the step
    anyway, so a step that stops answering costs its deadline and not the whole
    hour the agent would otherwise wait.

    It also says what the step was doing when it stopped, which is the part the
    step cannot do for itself: it reads the crumb file above and the output file
    of the command the step is running, and posts both as commit statuses, which
    the commit keeps whether the run ends or not. That is
    scripts/step_observer.ps1's whole job, and it is where the posting lives
    rather than here, so nothing the step calls has to reach the network.

    It is started with the launcher above, so it inherits none of this process's
    handles and cannot hold this step's output open itself; it is in the step's
    job, so it goes away with the step when the step ends on its own.
#>
function Start-StepWatchdog {
    param([int]$Minutes, [int]$StepProcessId = $PID, [string]$CrumbPath = "")

    if (-not $CrumbPath) {
        $CrumbPath = Join-Path ([System.IO.Path]::GetTempPath()) ("nano-step-{0}.crumbs" -f [guid]::NewGuid().ToString("n"))
    }
    $script:StepCrumbPath = $CrumbPath
    New-Item -ItemType File -Force -Path $CrumbPath | Out-Null
    $observer = Join-Path $PSScriptRoot "step_observer.ps1"
    $handle = [NanoStepCommand]::Start(
        "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$observer`" -StepProcessId $StepProcessId -Minutes $Minutes -CrumbPath `"$CrumbPath`"",
        (Get-Location).ProviderPath)
    [NanoStepCommand]::Release($handle)
    Write-Output "step watchdog: this step ends in $Minutes minute(s) whether it answers or not"
    Write-Output "step watchdog: it posts what the step writes to $CrumbPath as commit statuses, where the run has a token"
}

<#
    Ends a process and everything it started, with a deadline of its own.

    `run_tests.ps1` and `run_e2e_setup.ps1` take a command that never ended down
    from their deadline branch, and that branch exists to end a step: nothing in
    it may wait without a bound of its own, or the branch that ends a wedged step
    becomes part of the wedge. `taskkill /T` is the one call there that waits on
    something else -- it walks a tree that may include the very process that is
    holding everything up -- so it is given a deadline here. A tree it could not
    finish off is left to the job above, which ends it when this script's own
    process does.
#>
function Stop-ProcessTree {
    param([int]$ProcessId, [int]$DeadlineSeconds = 120)

    $killer = New-Object System.Diagnostics.ProcessStartInfo
    $killer.FileName = "taskkill.exe"
    $killer.Arguments = "/PID $ProcessId /T /F"
    $killer.UseShellExecute = $false
    $killer.CreateNoWindow = $true
    $killer.RedirectStandardOutput = $true
    $killer.RedirectStandardError = $true
    try {
        $victim = [System.Diagnostics.Process]::Start($killer)
    }
    catch {
        Write-Output ("  | the process tree could not be taken down: {0}" -f $_.Exception.Message)
        return
    }
    $ended = $victim.WaitForExit($DeadlineSeconds * 1000)
    $said = ""
    try {
        $said = $victim.StandardOutput.ReadToEnd() + $victim.StandardError.ReadToEnd()
    }
    catch {
        $said = ""
    }
    foreach ($line in @($said -split "\r?\n")) {
        if ($line) {
            Write-Output ("  | {0}" -f $line)
        }
    }
    if (-not $ended) {
        Write-Output ("  | taskkill did not end within {0} second(s); the job this step joined ends the tree" -f $DeadlineSeconds)
    }
}
