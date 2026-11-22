namespace MyRigConfig.Core.Host;

public enum PortCheckResult { Available, Unchanged, OutOfRange, InUse }

/// <summary>Result of checking the port the user typed. <see cref="Owner"/> names the program using
/// it.</summary>
public sealed partial record PortCheck(PortCheckResult Result, int Port, string? Owner = null);
