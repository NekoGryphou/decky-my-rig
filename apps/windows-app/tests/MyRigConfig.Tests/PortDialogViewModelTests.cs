using MyRigConfig.Features.Port;

namespace MyRigConfig.Tests;

[TestClass]
public sealed class PortDialogViewModelTests
{
    [TestMethod]
    public async Task NativeLookupLeavesUiResponsiveAndCoalescesToLatestInput()
    {
        using var ui = new PortUiContext();
        using var lookup = new BlockingPortLookup();
        var ready = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var published = new List<int>();
        PortDialogViewModel? model = null;
        await ui.InvokeAsync(() =>
        {
            model = new PortDialogViewModel(47991, lookup.Find, _ => null);
            model.PropertyChanged += (_, args) =>
            {
                if (args.PropertyName != nameof(model.CanSave) || !model.CanSave) return;
                published.Add(model.ChosenPort);
                if (model.ChosenPort == 50000) ready.TrySetResult();
            };
        });
        var changing = ui.InvokeAsync(() => model!.PortInput = 48000);
        Task? editing = null;
        var responsive = false;
        try
        {
            await lookup.Entered.Task.WaitAsync(TimeSpan.FromSeconds(3));
            editing = ui.InvokeAsync(() => { model!.PortInput = 49000; model.PortInput = 50000; });
            try { await editing.WaitAsync(TimeSpan.FromSeconds(1)); responsive = true; }
            catch (TimeoutException) { }
            lookup.Release();
            await changing;
            await editing;
            if (responsive) await ready.Task.WaitAsync(TimeSpan.FromSeconds(3));
            Assert.IsTrue(responsive, "Port lookup must not block another UI action.");
            Assert.AreEqual(1, lookup.Maximum);
            CollectionAssert.AreEqual(new[] { 48000, 50000 }, lookup.Queries);
            CollectionAssert.AreEqual(new[] { 50000 }, published);
        }
        finally
        {
            lookup.Release();
            await changing;
            if (editing is not null) await editing;
            await ui.InvokeAsync(() => model!.CloseAsync());
        }
    }

    [TestMethod]
    public async Task ClosingJoinsLookupWithoutPublishingItsLateResult()
    {
        using var ui = new PortUiContext();
        using var release = new ManualResetEventSlim();
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        PortDialogViewModel? model = null;
        await ui.InvokeAsync(() => model = new PortDialogViewModel(47991, _ =>
        {
            entered.TrySetResult();
            release.Wait();
            return null;
        }, _ => null));
        var changing = ui.InvokeAsync(() => model!.PortInput = 48000);
        Task? closing = null;
        try
        {
            await entered.Task.WaitAsync(TimeSpan.FromSeconds(3));
            closing = ui.InvokeAsync(() => model!.CloseAsync());
            Assert.IsFalse(closing.IsCompleted);
            release.Set();
            await changing;
            await closing;
            await ui.InvokeAsync(() => Assert.IsFalse(model!.CanSave));
        }
        finally
        {
            release.Set();
            await changing;
            if (closing is not null) await closing;
            await ui.InvokeAsync(() => model!.CloseAsync());
        }
    }

    [TestMethod]
    [DataRow(double.NaN)]
    [DataRow(1023d)]
    [DataRow(65536d)]
    [DataRow(48000.5d)]
    public async Task InvalidInputFailsWithoutNativeLookup(double input)
    {
        using var ui = new PortUiContext();
        PortDialogViewModel? model = null;
        var rejected = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var calls = 0;
        await ui.InvokeAsync(() =>
        {
            model = new PortDialogViewModel(47991, _ => { Interlocked.Increment(ref calls); return null; }, _ => null);
            model.PropertyChanged += (_, args) => { if (args.PropertyName == nameof(model.Error) && model.HasError) rejected.TrySetResult(); };
            model.PortInput = input;
        });
        try
        {
            await rejected.Task.WaitAsync(TimeSpan.FromSeconds(3));
            await ui.InvokeAsync(() => { Assert.IsFalse(model!.CanSave); Assert.AreEqual(0, model.ChosenPort); Assert.AreEqual(0, calls); });
        }
        finally { await ui.InvokeAsync(() => model!.CloseAsync()); }
    }

    [TestMethod]
    public async Task OccupiedPortShowsTheNativeOwnerAndCannotBeSaved()
    {
        using var ui = new PortUiContext();
        PortDialogViewModel? model = null;
        var rejected = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        await ui.InvokeAsync(() =>
        {
            model = new PortDialogViewModel(47991, port => port == 48000 ? 42 : null, pid => pid == 42 ? "OtherApp" : "Unexpected");
            model.PropertyChanged += (_, args) => { if (args.PropertyName == nameof(model.Error) && model.HasError) rejected.TrySetResult(); };
            model.PortInput = 48000;
        });
        try
        {
            await rejected.Task.WaitAsync(TimeSpan.FromSeconds(3));
            await ui.InvokeAsync(() =>
            {
                Assert.IsFalse(model!.CanSave);
                Assert.IsTrue(model.Error.Contains("48000", StringComparison.Ordinal));
                Assert.IsTrue(model.Error.Contains("OtherApp", StringComparison.Ordinal));
            });
        }
        finally { await ui.InvokeAsync(() => model!.CloseAsync()); }
    }
    [TestMethod]
    public async Task FailedNativeLookupReportsErrorWithoutEnablingSave()
    {
        using var ui = new PortUiContext();
        PortDialogViewModel? model = null;
        var rejected = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        try
        {
            await ui.InvokeAsync(() =>
            {
                model = new PortDialogViewModel(47991, _ => throw new System.ComponentModel.Win32Exception(5), _ => null);
                model.PropertyChanged += (_, args) => { if (args.PropertyName == nameof(model.Error) && model.HasError) rejected.TrySetResult(); };
                model.PortInput = 48000;
            });
            await rejected.Task.WaitAsync(TimeSpan.FromSeconds(3));
            await ui.InvokeAsync(() => Assert.IsFalse(model!.CanSave));
        }
        finally { if (model is not null) await ui.InvokeAsync(() => model.CloseAsync()); }
    }
}
