using MyRigConfig.Features.Host;
using MyRigConfig.Shared;

namespace MyRigConfig.Tests;

[TestClass]
public sealed class HostCommandFailureTests
{
    private sealed class ThrowingDialogs(Exception failure) : IHostDialogs
    {
        public Task ShowPairingAsync(CancellationToken cancellationToken) => Task.FromException(failure);
        public Task<bool> ConfirmRemovalAsync(string name) => Task.FromException<bool>(failure);
        public Task<string?> AskNameAsync(string current) => Task.FromException<string?>(failure);
        public Task<int?> AskPortAsync(int current) => Task.FromException<int?>(failure);
    }

    private static async Task<(HostViewModel Model, List<Exception> Logged)> Model(Exception failure)
    {
        var model = TestHost.Model(TestHost.Client(_ => TestHost.ListJson((TestHost.DeviceIdText, "Steam Deck"))));
        var logged = new List<Exception>();
        model.LogError = logged.Add;
        model.Dialogs = new ThrowingDialogs(failure);
        await model.RefreshCommand.ExecuteAsync(null);
        return (model, logged);
    }

    [TestMethod]
    [DataRow("pair")]
    [DataRow("remove")]
    [DataRow("rename")]
    [DataRow("port")]
    public async Task AnUnexpectedExceptionFromADialogCommandIsLoggedAndShownAsAFailure(string name)
    {
        var failure = new InvalidOperationException("Fixture bug.");
        var (model, logged) = await Model(failure);
        var row = model.PairedDevices.Single();
        await (name switch
        {
            "pair" => model.PairCommand.ExecuteAsync(null),
            "remove" => model.RemoveCommand.ExecuteAsync(row.DeviceId),
            "rename" => model.RenameCommand.ExecuteAsync(row.DeviceId),
            _ => model.ChangePortCommand.ExecuteAsync(null),
        });
        CollectionAssert.AreEqual(new[] { failure }, logged);
        Assert.IsTrue(model.IsFeedbackOpen);
        Assert.AreEqual(MessageTone.Error, model.FeedbackSeverity);
        Assert.AreEqual(name switch
        {
            "pair" => HostMessages.PairFailed,
            "remove" => HostMessages.RemovalUncertain,
            "rename" => HostMessages.RenameUncertain,
            _ => HostMessages.PortNotChanged,
        }, model.FeedbackMessage);
        Assert.IsFalse(model.IsDialogOpen, "the next dialog command must be able to run");
        Assert.IsFalse(model.IsInteracting);
    }

    [TestMethod]
    public async Task ACancellationFromADialogCommandIsNotAFailure()
    {
        var (model, logged) = await Model(new OperationCanceledException());
        await model.PairCommand.ExecuteAsync(null);
        Assert.AreEqual(0, logged.Count);
        Assert.IsFalse(model.IsFeedbackOpen);
    }
}
