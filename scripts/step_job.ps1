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

# The handle is kept for the whole life of the script: the job is over once the
# last handle to it closes, and the process holding this one is the step itself.
$script:StepJobHandle = [IntPtr]::Zero

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
