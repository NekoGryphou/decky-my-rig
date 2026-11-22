using System.Net;
using System.Net.Sockets;
using System.Runtime.Versioning;
using MyRigConfig.Core.Host;

namespace MyRigConfig.Tests;

[TestClass]
[SupportedOSPlatform("windows10.0")]
public sealed class HostNetworkTests
{
    [TestMethod]
    public void ListenerIsAttributedToItsProcessAndReleasedWhenClosed()
    {
        var listener = new TcpListener(IPAddress.Any, 0);
        listener.Start();
        var port = ((IPEndPoint)listener.LocalEndpoint).Port;
        try
        {
            Assert.AreEqual(Environment.ProcessId, HostNetwork.FindListener(port));
            Assert.IsNull(HostDiagnostics.FindConflict(port, Environment.ProcessId), "the service's own listener is not a conflict");
            var conflict = HostDiagnostics.FindConflict(port, serviceProcessId: null);
            Assert.AreEqual(port, conflict?.Port);
            Assert.AreEqual(HostNetwork.ProcessName(Environment.ProcessId), conflict?.ProcessName);
        }
        finally { listener.Stop(); }
        Assert.IsNull(HostNetwork.FindListener(port));
    }

    [TestMethod]
    public void Ipv6OnlyListenerIsFound()
    {
        var listener = new TcpListener(IPAddress.IPv6Any, 0);
        listener.Server.DualMode = false;
        listener.Start();
        try { Assert.AreEqual(Environment.ProcessId, HostNetwork.FindListener(((IPEndPoint)listener.LocalEndpoint).Port)); }
        finally { listener.Stop(); }
    }

    [TestMethod]
    public async Task OtherIpv6OwnerIsReportedWhenOurIpv4ListenerUsesTheSamePort()
    {
        using var own = new TcpListener(IPAddress.Loopback, 0);
        own.Start();
        var port = ((IPEndPoint)own.LocalEndpoint).Port;
        using var other = await HostNetworkFixture.StartAsync(port);
        var conflict = HostDiagnostics.FindConflict(port, Environment.ProcessId);
        Assert.IsNotNull(conflict, "our IPv4 listener must not hide another process's IPv6 listener");
        Assert.AreEqual(port, conflict.Port);
        Assert.AreEqual(HostNetwork.ProcessName(other.ProcessId), conflict.ProcessName);
    }

    [TestMethod]
    public void PortCheckClassifiesTypedPorts()
    {
        Assert.AreEqual(PortCheckResult.OutOfRange, PortCheck.Of(double.NaN, 47991).Result);
        Assert.AreEqual(PortCheckResult.OutOfRange, PortCheck.Of(1023, 47991).Result);
        Assert.AreEqual(PortCheckResult.OutOfRange, PortCheck.Of(65536, 47991).Result);
        Assert.AreEqual(PortCheckResult.OutOfRange, PortCheck.Of(48000.5, 47991).Result);
        Assert.AreEqual(PortCheckResult.Unchanged, PortCheck.Of(47991, 47991).Result);

        var listener = new TcpListener(IPAddress.Any, 0);
        listener.Start();
        var port = ((IPEndPoint)listener.LocalEndpoint).Port;
        try
        {
            var taken = PortCheck.Of(port, 47991);
            Assert.AreEqual(PortCheckResult.InUse, taken.Result);
            Assert.AreEqual(HostNetwork.ProcessName(Environment.ProcessId), taken.Owner);
        }
        finally { listener.Stop(); }
        Assert.AreEqual(PortCheckResult.Available, PortCheck.Of(port, 47991).Result);
    }
}
