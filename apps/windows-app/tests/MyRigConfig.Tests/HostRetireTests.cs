using MyRigConfig.Core.LocalControl;
using MyRigConfig.Features.Host;
using MyRigConfig.Features.Pairing;

namespace MyRigConfig.Tests;

[TestClass]
public sealed class HostRetireTests
{
    private static (HostViewModel Model, List<string> Operations) Model()
    {
        var operations = new List<string>();
        var model = TestHost.Model(TestHost.Client(request =>
        {
            var operation = request.Contains("\"open_pairing\"", StringComparison.Ordinal) ? "open_pairing"
                : request.Contains("\"close_pairing\"", StringComparison.Ordinal) ? "close_pairing" : "list";
            operations.Add(operation);
            return operation switch
            {
                "open_pairing" => $"{{{TestHost.Envelope()},\"code\":\"00123456\",\"expires_in_seconds\":120}}",
                "close_pairing" => $"{{{TestHost.Envelope()}}}",
                _ => null,
            };
        }));
        return (model, operations);
    }

    [TestMethod]
    public async Task RetiringWhileACodeShowsClosesItOnTheHost()
    {
        var (model, operations) = Model();
        var pairing = new PairingViewModel(model);
        await pairing.StartAsync(CancellationToken.None);
        await pairing.CreateCodeAsync(CancellationToken.None);
        Assert.AreEqual(PairingStep.Code, pairing.Step);
        await model.RetireAsync();
        Assert.AreEqual(1, operations.Count(o => o == "close_pairing"));
    }

    [TestMethod]
    public async Task RetiringAfterTheDialogClosedSendsNoSecondClose()
    {
        var (model, operations) = Model();
        var pairing = new PairingViewModel(model);
        await pairing.StartAsync(CancellationToken.None);
        await pairing.CreateCodeAsync(CancellationToken.None);
        await pairing.CloseAsync();
        await model.RetireAsync();
        Assert.AreEqual(1, operations.Count(o => o == "close_pairing"));
    }

    [TestMethod]
    public async Task RetiringWhileTheOpenRequestIsInFlightStillClosesTheCode()
    {
        var operations = new List<string>();
        var sending = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var model = TestHost.Model(new HostControlClientService(async (request, onSending, token) =>
        {
            onSending();
            var text = System.Text.Encoding.UTF8.GetString(request);
            if (text.Contains("\"open_pairing\"", StringComparison.Ordinal))
            {
                operations.Add("open_pairing");
                sending.SetResult();
                await Task.Delay(Timeout.InfiniteTimeSpan, token);
            }
            operations.Add(text.Contains("\"close_pairing\"", StringComparison.Ordinal) ? "close_pairing" : "list");
            return TestHost.Bytes(text.Contains("\"close_pairing\"", StringComparison.Ordinal)
                ? $"{{{TestHost.Envelope()}}}" : TestHost.ListJson());
        }));
        await model.RefreshCommand.ExecuteAsync(null);
        using var opening = new CancellationTokenSource();
        var open = model.OpenPairingAsync(TestHost.Id, "Steam Deck", opening.Token);
        await sending.Task;
        await model.RetireAsync();
        opening.Cancel();
        await Assert.ThrowsExceptionAsync<OperationCanceledException>(() => open);
        Assert.AreEqual(1, operations.Count(o => o == "close_pairing"));
    }

    [TestMethod]
    public async Task AFailedCloseIsTriedAgainWhenThePageRetires()
    {
        var failClose = true;
        var closes = 0;
        var model = TestHost.Model(new HostControlClientService((request, onSending, _) =>
        {
            onSending();
            var text = System.Text.Encoding.UTF8.GetString(request);
            if (text.Contains("\"close_pairing\"", StringComparison.Ordinal))
            {
                closes++;
                if (failClose) throw new IOException("Fixture pipe unavailable.");
                return Task.FromResult(TestHost.Bytes($"{{{TestHost.Envelope()}}}"));
            }
            return Task.FromResult(TestHost.Bytes(text.Contains("\"open_pairing\"", StringComparison.Ordinal)
                ? $"{{{TestHost.Envelope()},\"code\":\"00123456\",\"expires_in_seconds\":120}}" : TestHost.ListJson()));
        }));
        var pairing = new PairingViewModel(model);
        await pairing.StartAsync(CancellationToken.None);
        await pairing.CreateCodeAsync(CancellationToken.None);
        await pairing.CloseAsync();
        Assert.AreEqual(1, closes);
        failClose = false;
        await model.RetireAsync();
        Assert.AreEqual(2, closes);
        await model.RetireAsync();
        Assert.AreEqual(2, closes, "a closed code is not closed twice");
    }

    [TestMethod]
    public async Task RetiringWithoutAnOpenCodeSendsNothing()
    {
        var (model, operations) = Model();
        await model.RetireAsync();
        Assert.IsFalse(operations.Contains("close_pairing"));
    }

    [TestMethod]
    public async Task RetiringCancelsTheRunningCommands()
    {
        var (model, _) = Model();
        var cancelled = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        model.Dialogs = new WaitingDialogs(cancelled);
        var pairing = model.PairCommand.ExecuteAsync(null);
        await model.RetireAsync();
        await cancelled.Task;
        await pairing;
    }

    private sealed class WaitingDialogs(TaskCompletionSource cancelled) : IHostDialogs
    {
        public async Task ShowPairingAsync(CancellationToken cancellationToken)
        {
            try { await Task.Delay(Timeout.InfiniteTimeSpan, cancellationToken); }
            catch (OperationCanceledException) { cancelled.SetResult(); throw; }
        }

        public Task<bool> ConfirmRemovalAsync(string name) => Task.FromResult(false);
        public Task<string?> AskNameAsync(string current) => Task.FromResult<string?>(null);
        public Task<int?> AskPortAsync(int current) => Task.FromResult<int?>(null);
    }
}
