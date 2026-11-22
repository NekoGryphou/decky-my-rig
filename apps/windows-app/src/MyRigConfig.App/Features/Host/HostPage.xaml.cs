using MyRigConfig.Shared;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using MyRigConfig.Features.Pairing;
using MyRigConfig.Features.Port;
using MyRigConfig.Features.Rename;

using static MyRigConfig.Features.Host.HostPageConstants;

namespace MyRigConfig.Features.Host;

public sealed partial class HostPage : Page, IHostDialogs
{
    private CancellationTokenSource? lifetime;
    private DispatcherQueueTimer? refreshTimer;

    public HostPage()
    {
        InitializeComponent();
        ViewModel.Dialogs = this;
    }

    public HostViewModel ViewModel { get; } = new();

    private async void Page_Loaded(object sender, RoutedEventArgs e)
    {
        lifetime = new CancellationTokenSource();
        App.Window.Closed += Window_Closed;
        App.Window.Activated += Window_Activated;
        refreshTimer = DispatcherQueue.CreateTimer();
        refreshTimer.Interval = RefreshInterval;
        refreshTimer.Tick += (_, _) => RefreshInBackground();
        refreshTimer.Start();
        await ViewModel.RefreshCommand.ExecuteAsync(null);
    }

    // ? Like Windows Settings, there's no Refresh button. The page refreshes when the window gets
    //   focus again, and every so often while it's open.
    private void Window_Activated(object sender, WindowActivatedEventArgs args)
    {
        if (args.WindowActivationState != WindowActivationState.Deactivated) RefreshInBackground();
    }

    private async void RefreshInBackground()
    {
        if (ViewModel.IsDialogOpen || lifetime is not { } owned) return;
        try { await ViewModel.RefreshQuietlyAsync(owned.Token); }
        catch (OperationCanceledException) { }
    }

    private void Page_Unloaded(object sender, RoutedEventArgs e) => Retire();

    private void Window_Closed(object sender, WindowEventArgs args) => Retire();

    private void Retire()
    {
        App.Window.Closed -= Window_Closed;
        App.Window.Activated -= Window_Activated;
        refreshTimer?.Stop();
        refreshTimer = null;
        ViewModel.RefreshCommand.Cancel();
        ViewModel.StartServiceCommand.Cancel();
        ViewModel.PairCommand.Cancel();
        ViewModel.RenameCommand.Cancel();
        ViewModel.RemoveCommand.Cancel();
        ViewModel.ChangePortCommand.Cancel();
        ViewModel.ChangeTestModeCommand.Cancel();
        ViewModel.RestartServiceCommand.Cancel();
        lifetime?.Cancel();
        lifetime?.Dispose();
        lifetime = null;
    }

    // ? ToggleSwitch has no Command, so the view model puts the confirmed value back. The switch is
    //   disabled while the change is saved, which takes its focus away, so keyboard users get their
    //   focus back afterwards.
    private async void TestMode_Toggled(object sender, RoutedEventArgs e)
    {
        if (sender is not ToggleSwitch toggle) return;
        var focused = toggle.FocusState != FocusState.Unfocused;
        await ViewModel.ChangeTestModeCommand.ExecuteAsync(toggle.IsOn);
        if (focused) toggle.Focus(FocusState.Programmatic);
    }

    public async Task ShowPairingAsync(CancellationToken cancellationToken)
    {
        var dialog = new PairingDialog(ViewModel, cancellationToken) { XamlRoot = XamlRoot };
        await dialog.ShowAsync();
    }

    public Task<bool> ConfirmRemovalAsync(string name) =>
        ConfirmAsync(HostMessages.RemoveDialogTitle(name), HostMessages.RemoveMessage, HostMessages.RemoveAction);

    public async Task<int?> AskPortAsync(int current)
    {
        var dialog = new PortDialog(current) { XamlRoot = XamlRoot };
        try
        {
            return await dialog.ShowAsync() == ContentDialogResult.Primary && dialog.ViewModel.CanSave
                ? dialog.ViewModel.ChosenPort
                : null;
        }
        finally { await dialog.ViewModel.CloseAsync(); }
    }

    public async Task<string?> AskNameAsync(string current)
    {
        var dialog = new RenameDialog(current) { XamlRoot = XamlRoot };
        return await dialog.ShowAsync() == ContentDialogResult.Primary && dialog.ViewModel.CanSave
            ? dialog.ViewModel.ChosenName
            : null;
    }

    private async Task<bool> ConfirmAsync(string title, string? message, string primary)
    {
        var dialog = new ContentDialog
        {
            XamlRoot = XamlRoot,
            Style = (Style)Application.Current.Resources["DefaultContentDialogStyle"],
            Title = title,
            Content = message,
            PrimaryButtonText = primary,
            CloseButtonText = SharedMessages.Cancel,
            DefaultButton = ContentDialogButton.Close,
        };
        return await dialog.ShowAsync() == ContentDialogResult.Primary;
    }
}
