using Microsoft.VisualStudio.TestTools.UnitTesting;
using System.Text;
using MyRigConfig.Core.Host;
using MyRigConfig.Core.LocalControl;
using MyRigConfig.Features.Host;

namespace MyRigConfig.Tests;

[TestClass]
public sealed class HostDeviceEditTests
{
    private const string Host = TestHost.IdText;
    private const string Device = TestHost.DeviceIdText;

    private static (HostViewModel Model, List<string> Requests) Model(Func<string> renameReply)
    {
        var name = "SteamDeck";
        var requests = new List<string>();
        var client = new HostControlClientService((request, sending, _) =>
        {
            sending();
            var text = Encoding.UTF8.GetString(request);
            requests.Add(text);
            if (text.Contains("\"remove\"", StringComparison.Ordinal))
                return Task.FromResult(Encoding.UTF8.GetBytes(
                    $"{{\"version\":1,\"ok\":true,\"host_id\":\"{Host}\",\"removed\":true}}"));
            if (text.Contains("\"rename\"", StringComparison.Ordinal))
            {
                var reply = renameReply();
                if (reply.Contains("\"renamed\":true", StringComparison.Ordinal)) name = "Living room device";
                return Task.FromResult(Encoding.UTF8.GetBytes(reply));
            }
            return Task.FromResult(Encoding.UTF8.GetBytes(
                $"{{\"version\":1,\"ok\":true,\"host_id\":\"{Host}\",\"paired_devices\":[{{\"device_id\":\"{Device}\",\"name\":\"{name}\"}}]}}"));
        });
        return (TestHost.Model(client), requests);
    }

    [TestMethod]
    public async Task RenameSendsTheNewNameAndShowsTheServicesList()
    {
        var (model, requests) = Model(() => $"{{\"version\":1,\"ok\":true,\"host_id\":\"{Host}\",\"renamed\":true}}");
        await model.RefreshCommand.ExecuteAsync(null);
        await model.RenameDeviceAsync(Guid.Parse(Device), current => Task.FromResult<string?>("Living room device"), CancellationToken.None);
        Assert.IsTrue(requests.Any(r => r.Contains("\"name\":\"Living room device\"", StringComparison.Ordinal)));
        Assert.AreEqual("Living room device", model.PairedDevices.Single().Name);
        Assert.AreEqual(HostMessages.Renamed("Living room device"), model.FeedbackMessage);
    }

    private sealed class Dialogs(string? name, bool confirm) : IHostDialogs
    {
        internal int Shown;
        public Task ShowPairingAsync(CancellationToken cancellationToken) { Shown++; return Task.CompletedTask; }
        public Task<bool> ConfirmRemovalAsync(string current) { Shown++; return Task.FromResult(confirm); }
        public Task<string?> AskNameAsync(string current) { Shown++; return Task.FromResult(name); }
        public Task<int?> AskPortAsync(int current) { Shown++; return Task.FromResult<int?>(null); }
    }

    [TestMethod]
    public async Task RowCommandsAskThroughThePageDialogs()
    {
        var (model, requests) = Model(() => $"{{\"version\":1,\"ok\":true,\"host_id\":\"{Host}\",\"renamed\":true}}");
        var dialogs = new Dialogs("Living room device", confirm: false);
        model.Dialogs = dialogs;
        await model.RefreshCommand.ExecuteAsync(null);
        var row = model.PairedDevices.Single();
        await row.RemoveCommand.ExecuteAsync(row.DeviceId);
        Assert.IsFalse(requests.Any(r => r.Contains("\"remove\"", StringComparison.Ordinal)));
        await row.RenameCommand.ExecuteAsync(row.DeviceId);
        Assert.AreEqual(2, dialogs.Shown);
        Assert.AreEqual("Living room device", model.PairedDevices.Single().Name);
        Assert.IsFalse(model.IsDialogOpen);
    }

    [TestMethod]
    public async Task CancelledOrUnchangedNameSendsNothing()
    {
        var (model, requests) = Model(() => throw new InvalidOperationException("must not rename"));
        await model.RefreshCommand.ExecuteAsync(null);
        await model.RenameDeviceAsync(Guid.Parse(Device), _ => Task.FromResult<string?>(null), CancellationToken.None);
        await model.RenameDeviceAsync(Guid.Parse(Device), current => Task.FromResult<string?>(current), CancellationToken.None);
        Assert.IsFalse(requests.Any(r => r.Contains("\"rename\"", StringComparison.Ordinal)));
    }

    [TestMethod]
    public async Task RenamingADeviceAlreadyGoneSaysSo()
    {
        // - renamed:false means the service says the device isn't paired anymore.
        var (model, _) = Model(() => $"{{\"version\":1,\"ok\":true,\"host_id\":\"{Host}\",\"renamed\":false}}");
        await model.RefreshCommand.ExecuteAsync(null);
        await model.RenameDeviceAsync(Guid.Parse(Device), _ => Task.FromResult<string?>("Living room device"), CancellationToken.None);
        Assert.AreEqual(HostMessages.NoLongerPaired, model.FeedbackMessage);
    }

    [TestMethod]
    public async Task UnconfirmedRenameSaysSo()
    {
        // - An unreadable reply: the rename may or may not have happened.
        var (model, _) = Model(() => "not a reply");
        await model.RefreshCommand.ExecuteAsync(null);
        await model.RenameDeviceAsync(Guid.Parse(Device), _ => Task.FromResult<string?>("Living room device"), CancellationToken.None);
        Assert.AreEqual(HostMessages.RenameUncertain, model.FeedbackMessage);
    }

    [TestMethod]
    public async Task RemovalNeedsConfirmation()
    {
        var (model, requests) = Model(() => throw new InvalidOperationException("must not rename"));
        await model.RefreshCommand.ExecuteAsync(null);
        await model.RemoveDeviceAsync(Guid.Parse(Device), _ => Task.FromResult(false), CancellationToken.None);
        Assert.IsFalse(requests.Any(r => r.Contains("\"remove\"", StringComparison.Ordinal)));
        await model.RemoveDeviceAsync(Guid.Parse(Device), _ => Task.FromResult(true), CancellationToken.None);
        Assert.IsTrue(requests.Any(r => r.Contains("\"remove\"", StringComparison.Ordinal)));
        Assert.AreEqual(HostMessages.Removed("SteamDeck"), model.FeedbackMessage);
    }
}
