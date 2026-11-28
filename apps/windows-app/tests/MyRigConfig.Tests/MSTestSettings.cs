// ? Tests run one at a time: they share process-wide state (HostServiceControlService.Pending,
//   the thread pool) and the suite takes well under a second, so parallelism buys nothing.
[assembly: DoNotParallelize]
[assembly: System.Runtime.Versioning.SupportedOSPlatform("windows10.0")]
