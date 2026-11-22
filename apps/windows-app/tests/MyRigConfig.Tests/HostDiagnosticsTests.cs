using MyRigConfig.Core.Host;
using MyRigConfig.Core.LocalControl;
using System.Runtime.Versioning;

namespace MyRigConfig.Tests;

[TestClass]
[SupportedOSPlatform("windows10.0")]
public sealed class HostDiagnosticsTests
{
    [TestMethod]
    public async Task RunningServiceReadsStatusThroughControlClient()
    {
        var host = Guid.Parse("03c85317-9d93-4fb3-a04b-91a0e16668c6");
        var queried = 0;
        var observation = await HostDiagnostics.ReadAsync(
            _ => Task.FromResult((ServiceState.Running, (int?)null)),
            _ => {
                queried++;
                return Task.FromResult<LocalControlReply>(new LocalControlStatusReply(host, 49000, true, "0.1.0", "listening"));
            },
            CancellationToken.None);
        Assert.AreEqual(1, queried);
        Assert.AreEqual(ServiceState.Running, observation.ServiceState);
        Assert.AreEqual(49000, observation.Status?.Port);
        Assert.IsTrue(observation.Status?.TestMode);
    }

    [TestMethod]
    public async Task StoppedServiceDoesNotQueryControlClient()
    {
        var observation = await HostDiagnostics.ReadAsync(
            _ => Task.FromResult((ServiceState.Stopped, (int?)null)),
            _ => throw new AssertFailedException("Stopped service must not be queried"),
            CancellationToken.None);
        Assert.IsNull(observation.Status);
    }
}
