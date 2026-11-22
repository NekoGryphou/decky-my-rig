namespace MyRigConfig.Core.Host;
public sealed class HostServiceException(Exception inner)
    : Exception("The service did not reach the requested state.", inner);
