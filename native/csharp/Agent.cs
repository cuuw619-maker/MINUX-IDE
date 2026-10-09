using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace MINUX.Agent;

public static class Agent
{
    // Exported through NativeAOT and linked into the final MINUX IDE executable.
    [UnmanagedCallersOnly(EntryPoint = "minux_agent_version", CallConvs = new[] { typeof(CallConvCdecl) })]
    public static int Version() => 1;
}
