using System.Diagnostics;

namespace MyRigConfig.Shared;

/// <summary>Records unexpected exceptions to the trace output. The user only sees a short failure message.</summary>
internal static class ErrorLogService
{
    internal static void Write(Exception exception) => Trace.TraceError("{0}", exception);
}
