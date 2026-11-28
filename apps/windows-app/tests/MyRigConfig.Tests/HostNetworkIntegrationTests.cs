using System.Net;
using System.Net.Sockets;
using System.Runtime.Versioning;
using MyRigConfig.Core.Host;

namespace MyRigConfig.Tests;

/// <summary>Reads the real Windows TCP tables. Excluded from the default run; use
/// <c>--filter TestCategory=Integration</c> to run it.</summary>
[TestClass]
[TestCategory("Integration")]
[SupportedOSPlatform("windows10.0")]
public sealed class HostNetworkIntegrationTests
{
    [TestMethod]
    public void AnIpv4ListenerIsAttributedToThisProcess()
    {
        using var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        var port = ((IPEndPoint)listener.LocalEndpoint).Port;
        Assert.AreEqual(Environment.ProcessId, HostNetworkNative.FindListener(port, excludedProcessId: null));
        Assert.IsNull(HostNetworkNative.FindListener(port, Environment.ProcessId), "an excluded process is not a listener");
    }

    [TestMethod]
    public void AnIpv6OnlyListenerIsFound()
    {
        using var listener = new TcpListener(IPAddress.IPv6Loopback, 0);
        listener.Server.DualMode = false;
        listener.Start();
        var port = ((IPEndPoint)listener.LocalEndpoint).Port;
        Assert.AreEqual(Environment.ProcessId, HostNetworkNative.FindListener(port, excludedProcessId: null));
    }
}
