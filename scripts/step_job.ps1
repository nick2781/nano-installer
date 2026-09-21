<#
    Puts the process that runs a build step into a job the system takes down with it.

    Every process a step starts inherits the step's own output, and a step is over
    only once that output has closed. One process left behind -- a wizard a
    failing case did not close, the toolchain's telemetry helper the linker starts
    -- therefore holds the step open: the step never ends, is archived with no log
    at all, and neither a timeout nor a cancel can end it.

    A process that joins a job hands that job to every process it starts
    afterwards, and a job created here is destroyed with the last handle to it,
    which is the one the joining script holds. Joining it therefore means that
    when the step's script exits, whatever it left behind goes with it and the
    step's output closes.

    Joining is best effort: a host may already run its steps in a job of its own
    that refuses a second one, and a step that cannot join still runs -- it only
    keeps the failure mode above. Either way the step says what happened, so a
    step that hangs anyway still reports whether the guard was in place.
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
