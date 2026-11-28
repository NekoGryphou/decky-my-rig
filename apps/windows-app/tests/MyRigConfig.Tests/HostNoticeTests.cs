using System.Text;
using MyRigConfig.Core.Host;
using MyRigConfig.Core.LocalControl;
using MyRigConfig.Features.Host;
using MyRigConfig.Shared;

namespace MyRigConfig.Tests;

[TestClass]
public sealed class HostNoticeTests
{
    private static HostViewModel Model(Func<byte[]> exchange) =>
        TestHost.Model(new HostControlClientService((_, sending, _) => { sending(); return Task.FromResult(exchange()); }));

    [TestMethod]
    public void NoticeShowsTheMostImportantCondition()
    {
        var conflict = new PortConflict(47991, "OtherApp");
        Assert.AreEqual(HostMessages.MissingTitle,
            HostHelpers.SelectNotice(ServiceState.NotInstalled, conflict, true, true, true, true)?.Title);
        Assert.AreEqual(HostMessages.PortInUse(47991, "OtherApp"),
            HostHelpers.SelectNotice(ServiceState.Stopped, conflict, true, false, false, true)?.Title);
        Assert.AreEqual(HostMessages.ListenerUnavailable,
            HostHelpers.SelectNotice(ServiceState.Running, null, true, true, true, true)?.Title);
        // - Just a title: the port setting is right below it.
        Assert.AreEqual("", HostHelpers.SelectNotice(ServiceState.Running, null, true, false, false, false)?.Message);
        Assert.AreEqual(HostMessages.StoppedTitle,
            HostHelpers.SelectNotice(ServiceState.Stopped, null, false, true, true, true)?.Title);
        Assert.AreEqual(HostMessages.SignOutTitle,
            HostHelpers.SelectNotice(ServiceState.Running, null, false, true, true, true)?.Title);
        Assert.AreEqual(HostMessages.ControlTitle,
            HostHelpers.SelectNotice(ServiceState.Running, null, false, false, true, true)?.Title);
        Assert.AreEqual(MessageTone.Informational,
            HostHelpers.SelectNotice(ServiceState.Running, null, false, false, false, true)?.Severity);
        // - Like the other notices, the test-mode one is only a title.
        Assert.AreEqual("", HostHelpers.SelectNotice(ServiceState.Running, null, false, false, false, true)?.Message);
        Assert.IsNull(HostHelpers.SelectNotice(ServiceState.Running, null, false, false, false, false));
    }

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
        var model = Model(() => TestHost.Bytes(TestHost.ListJson()));
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

    [TestMethod]
    [DataRow(typeof(IOException))]
    [DataRow(typeof(System.ComponentModel.Win32Exception))]
    [DataRow(typeof(HostControlException))]
    public async Task ADiagnosticsFailureOfTheHostKindMeansUnavailable(Type failure)
    {
        var model = TestHost.Model(TestHost.Client(_ => null),
            _ => throw (Exception)(failure == typeof(HostControlException)
                ? new HostControlException(false) : Activator.CreateInstance(failure)!));
        await model.RefreshCommand.ExecuteAsync(null);
        Assert.AreEqual(ServiceState.Unavailable, model.ServiceState);
        Assert.IsFalse(model.IsBusy);
    }

    [TestMethod]
    public async Task AProgrammingErrorInDiagnosticsIsNotHiddenAsUnavailable()
    {
        var model = TestHost.Model(TestHost.Client(_ => null),
            _ => throw new InvalidOperationException("Fixture bug."));
        await Assert.ThrowsExceptionAsync<InvalidOperationException>(() => model.RefreshCommand.ExecuteAsync(null));
        Assert.AreNotEqual(ServiceState.Unavailable, model.ServiceState);
        Assert.IsFalse(model.IsBusy, "the failed refresh must release the page");
        Assert.IsFalse(model.IsInteracting);
    }
}
