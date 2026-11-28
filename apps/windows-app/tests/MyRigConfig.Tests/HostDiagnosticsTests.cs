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
        var host = TestHost.Id;
        var queried = 0;
        var observation = await HostDiagnosticsService.ReadAsync(
            _ => Task.FromResult((ServiceState.Running, (int?)null)),
            _ => {
                queried++;
                return Task.FromResult<LocalControlReply>(new LocalControlStatusReply(host, 49000, true, "0.1.0", "listening"));
            },
            new StubPortProbe(_ => null), CancellationToken.None);
        Assert.AreEqual(1, queried);
        Assert.AreEqual(ServiceState.Running, observation.ServiceState);
        Assert.AreEqual(49000, observation.Status?.Port);
        Assert.IsTrue(observation.Status?.TestMode);
    }

    [TestMethod]
    public async Task StoppedServiceDoesNotQueryControlClient()
    {
        var observation = await HostDiagnosticsService.ReadAsync(
            _ => Task.FromResult((ServiceState.Stopped, (int?)null)),
            _ => throw new AssertFailedException("Stopped service must not be queried"),
            new StubPortProbe(_ => null), CancellationToken.None);
        Assert.IsNull(observation.Status);
    }

    [TestMethod]
    public void AnotherProcessOnTheServicePortIsAConflictAndOurOwnIsNot()
    {
        var probe = new StubPortProbe(port => port == 49000 ? 42 : null, pid => pid == 42 ? "OtherApp" : null);
        var conflict = HostDiagnosticsService.FindConflict(probe, 49000, serviceProcessId: null);
        Assert.AreEqual(new PortConflict(49000, "OtherApp"), conflict);
        Assert.IsNull(HostDiagnosticsService.FindConflict(probe, 49000, serviceProcessId: 42));
    }

    [TestMethod]
    public void AnUnreadablePortTableIsNoConflict()
    {
        var probe = new StubPortProbe(_ => throw new System.ComponentModel.Win32Exception(5));
        Assert.IsNull(HostDiagnosticsService.FindConflict(probe, 49000, serviceProcessId: null));
    }

    [TestMethod]
    public async Task AConflictOnTheReportedPortShowsInTheObservation()
    {
        var probe = new StubPortProbe(port => port == 49000 ? 42 : null, _ => "OtherApp");
        var observation = await HostDiagnosticsService.ReadAsync(
            _ => Task.FromResult((ServiceState.Running, (int?)7)),
            _ => Task.FromResult<LocalControlReply>(new LocalControlStatusReply(TestHost.Id, 49000, false, "0.1.0", "listening")),
            probe, CancellationToken.None);
        Assert.AreEqual(new PortConflict(49000, "OtherApp"), observation.PortConflict);
    }
}
