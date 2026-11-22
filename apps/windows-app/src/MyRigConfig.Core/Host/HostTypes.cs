namespace MyRigConfig.Core.Host;

public enum ServiceState { Checking, Running, Stopped, Starting, Stopping, NotInstalled, Unavailable }

public sealed record HostStatus(int Port, bool TestMode, string ListenerState);

public sealed record HostObservation(ServiceState ServiceState, HostStatus? Status,
    PortConflict? PortConflict);

/// <summary>Another program listens on the configured port. Its name is null when Windows hides it.</summary>
public sealed record PortConflict(int Port, string? ProcessName);
