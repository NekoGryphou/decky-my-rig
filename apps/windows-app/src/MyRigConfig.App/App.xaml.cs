using Microsoft.UI.Xaml;
using Microsoft.Windows.AppLifecycle;
using MyRigConfig.Shared;
using MyRigConfig.Shell;

namespace MyRigConfig;

public partial class App : Application
{
    /// <summary>The app's only window. Pages use it for lifetime events.</summary>
    public static Window Window { get; private set; } = null!;

    public App()
    {
        InitializeComponent();
        UnhandledException += (_, args) =>
        {
            ErrorLogService.Write(args.Exception);
            args.Handled = ShellHelpers.IsHarmless(args.Exception);
        };
    }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        Window = new MainWindow();
        Window.Activate();
        // * A second launch is redirected here, so bring this window up instead.
        AppInstance.GetCurrent().Activated += (_, _) =>
            Window.DispatcherQueue.TryEnqueue(() =>
            {
                if (ShellHelpers.ShouldBringToFront(Window is MainWindow { IsClosing: true }))
                    ProgramNative.BringToFront(Window);
            });
    }
}
