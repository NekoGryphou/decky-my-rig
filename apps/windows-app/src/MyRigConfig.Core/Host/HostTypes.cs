namespace MyRigConfig.Core.Host;

public enum ServiceState { Checking, Running, Stopped, Starting, Stopping, NotInstalled, Unavailable }

public sealed record HostStatus(int Port, bool TestMode, string ListenerState);

public sealed record HostObservation(ServiceState ServiceState, HostStatus? Status,
    PortConflict? PortConflict);

/// <summary>Another program listens on the configured port. Its name is null when Windows hides it.</summary>
public sealed record PortConflict(int Port, string? ProcessName);

/// <summary>Which process listens on a TCP port. The Windows implementation reads the TCP tables;
/// tests give their own answers.</summary>
internal interface IPortProbe
{
    /// <summary>ID of a process other than <paramref name="excludedProcessId"/> listening on
    /// <paramref name="port"/>, or null if there is none. Throws <see cref="System.ComponentModel.Win32Exception"/>
    /// when the table can't be read.</summary>
    int? FindListener(int port, int? excludedProcessId);

    /// <summary>A process's executable name, or null if Windows won't tell us.</summary>
    string? ProcessName(int processId);
}
