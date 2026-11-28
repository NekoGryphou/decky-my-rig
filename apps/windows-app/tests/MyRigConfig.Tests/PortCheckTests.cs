using MyRigConfig.Core.Host;

namespace MyRigConfig.Tests;

[TestClass]
public sealed class PortCheckTests
{
    private const int Current = 47991;

    [TestMethod]
    [DataRow(double.NaN)]
    [DataRow(1023d)]
    [DataRow(65536d)]
    [DataRow(48000.5d)]
    public void OutOfRangeInputIsRejectedWithoutAskingTheProbe(double input)
    {
        var probe = new StubPortProbe(_ => null);
        Assert.AreEqual(PortCheckResult.OutOfRange, PortCheck.Of(input, Current, probe).Result);
        Assert.AreEqual(0, probe.Queries.Length);
    }

    [TestMethod]
    public void TheCurrentPortIsUnchangedWithoutAskingTheProbe()
    {
        var probe = new StubPortProbe(_ => 42);
        Assert.AreEqual(PortCheckResult.Unchanged, PortCheck.Of(Current, Current, probe).Result);
        Assert.AreEqual(0, probe.Queries.Length);
    }

    [TestMethod]
    public void AListenedPortIsInUseAndNamesItsOwner()
    {
        var probe = new StubPortProbe(port => port == 48000 ? 42 : null, pid => pid == 42 ? "OtherApp" : null);
        var taken = PortCheck.Of(48000, Current, probe);
        Assert.AreEqual(PortCheckResult.InUse, taken.Result);
        Assert.AreEqual("OtherApp", taken.Owner);
        Assert.AreEqual(PortCheckResult.Available, PortCheck.Of(48001, Current, probe).Result);
    }
}
