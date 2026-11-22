using System.Text;
using MyRigConfig.Core.Host;
using MyRigConfig.Core.LocalControl;
using MyRigConfig.Features.Host;
using MyRigConfig.Shared;

namespace MyRigConfig.Tests;

[TestClass]
public sealed class HostNoticeTests
{
    private static readonly Guid HostId = Guid.Parse("03c85317-9d93-4fb3-a04b-91a0e16668c6");

    private static HostViewModel Model(Func<byte[]> exchange) =>
        new(new HostControlClient((_, sending, _) => { sending(); return Task.FromResult(exchange()); }),
            _ => Task.FromResult(new HostObservation(ServiceState.Running,
                new HostStatus(47991, false, "listening"), null)),
            TimeProvider.System);

    [TestMethod]
    public async Task UnauthorizedReplyAsksToSignOut()
    {
        var model = Model(() => Encoding.UTF8.GetBytes("{\"version\":1,\"ok\":false,\"error\":\"unauthorized\"}"));
        await model.RefreshCommand.ExecuteAsync(null);
        Assert.AreEqual(HostMessages.SignOutTitle, model.NoticeTitle);
        Assert.IsFalse(model.ShowNoDevices);
    }

    [TestMethod]
    public async Task DeniedPipeAsksToSignOut()
    {
        var model = Model(() => throw new HostControlException(false, unauthorized: true));
        await model.RefreshCommand.ExecuteAsync(null);
        Assert.AreEqual(HostMessages.SignOutTitle, model.NoticeTitle);
    }

    [TestMethod]
    public async Task AnUnreachableServiceShowsOnlyItsNotice()
    {
        // - The page notice says why the list is empty; a second bar would just repeat it.
        var model = Model(() => throw new HostControlException(false));
        await model.RefreshCommand.ExecuteAsync(null);
        Assert.AreEqual(HostMessages.ControlTitle, model.NoticeTitle);
        Assert.IsFalse(model.ShowNoDevices);
    }

    [TestMethod]
    public async Task EmptyListSaysNoPairedDevice()
    {
        var model = Model(() => Encoding.UTF8.GetBytes(
            "{\"version\":1,\"ok\":true,\"host_id\":\"03c85317-9d93-4fb3-a04b-91a0e16668c6\",\"paired_devices\":[]}"));
        await model.RefreshCommand.ExecuteAsync(null);
        Assert.IsFalse(model.IsNoticeOpen);
        Assert.IsTrue(model.ShowNoDevices);
        Assert.AreEqual(HostMessages.NoPairedDevice, model.NoDevicesText);
    }

    [TestMethod]
    public async Task ANewMessageReopensTheBarSoNarratorReadsIt()
    {
        // ? Narrator announces an InfoBar when it opens, not when its text changes.
        var model = Model(() => throw new HostControlException(false));
        await model.RefreshCommand.ExecuteAsync(null);
        var feedback = new List<bool>();
        var notice = new List<bool>();
        model.PropertyChanged += (_, args) =>
        {
            if (args.PropertyName == nameof(model.IsFeedbackOpen)) feedback.Add(model.IsFeedbackOpen);
            if (args.PropertyName == nameof(model.IsNoticeOpen)) notice.Add(model.IsNoticeOpen);
        };
        model.ShowFeedback("First", MessageTone.Error);
        model.ShowFeedback("Second", MessageTone.Error);
        CollectionAssert.AreEqual(new[] { true, false, true }, feedback);
        // - The notice changes from "can't reach the service" to "stopped".
        model.ServiceState = ServiceState.Stopped;
        CollectionAssert.AreEqual(new[] { false, true }, notice);
    }
}
