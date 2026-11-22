namespace MyRigConfig.Core.Host;

public static class HostConstants
{
    internal const int ServiceDoesNotExist = 1060;
    public const string ServiceName = "MyRigHost";
    internal static readonly TimeSpan ServiceStateDeadline = TimeSpan.FromSeconds(30);
}
