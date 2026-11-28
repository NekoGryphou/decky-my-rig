using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using MyRigConfig.Core.Host;
using MyRigConfig.Features.Host;
using Windows.Graphics;

using static MyRigConfig.Shell.MainWindowConstants;

namespace MyRigConfig.Shell;

public sealed partial class MainWindow : Window
{
    private bool closing;

    /// <summary>True once the window is hidden and waiting for a service action to finish.</summary>
    internal bool IsClosing => closing;

    public MainWindow()
    {
        InitializeComponent();
        ExtendsContentIntoTitleBar = true;
        SetTitleBar(AppTitleBar);

        // - Caption buttons follow the app theme, High Contrast changes included.
        var root = (FrameworkElement)Content;
        void UpdateCaptionTheme() => AppWindow.TitleBar.PreferredTheme = root.ActualTheme == ElementTheme.Light
            ? TitleBarTheme.Light : TitleBarTheme.Dark;
        root.ActualThemeChanged += (_, _) => UpdateCaptionTheme();
        UpdateCaptionTheme();

        AppWindow.SetIcon(ShellHelpers.IconPath(AppContext.BaseDirectory));
        var scale = GetDpiForWindow(WinRT.Interop.WindowNative.GetWindowHandle(this)) / 96.0;
        AppWindow.Resize(new SizeInt32((int)(DefaultWidth * scale), (int)(DefaultHeight * scale)));
        if (AppWindow.Presenter is OverlappedPresenter presenter)
        {
            presenter.PreferredMinimumWidth = (int)(MinimumWidth * scale);
            presenter.PreferredMinimumHeight = (int)(MinimumHeight * scale);
        }

        AppWindow.Closing += AppWindow_Closing;
        RootFrame.Navigate(typeof(HostPage));
    }

    // ! A restart cut in half (stopped but not started yet) would leave the PC unreachable, so the window hides and
    // ! only closes once the action has finished.
    private async void AppWindow_Closing(AppWindow sender, AppWindowClosingEventArgs args)
    {
        var pending = HostServiceControlService.Pending;
        if (pending.IsCompleted) return;
        args.Cancel = true;
        if (closing) return;
        closing = true;
        sender.Hide();
        await pending;
        Close();
    }
}
