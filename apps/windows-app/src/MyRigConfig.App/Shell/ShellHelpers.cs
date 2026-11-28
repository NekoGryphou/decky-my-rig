namespace MyRigConfig.Shell;

internal static class ShellHelpers
{
    /// <summary>The icon sits next to the app, so it is found wherever the app was started from.</summary>
    internal static string IconPath(string baseDirectory) => Path.Combine(baseDirectory, MainWindowConstants.IconFolder, MainWindowConstants.IconFileName);

    /// <summary>A window that is closing stays hidden: showing it again would let the user act on a
    /// service action that is only waiting to finish.</summary>
    internal static bool ShouldBringToFront(bool windowClosing) => !windowClosing;

    /// <summary>A cancellation escaping an async void handler, such as a page torn down mid-operation,
    /// is expected and does not crash the app. Any other exception ends it.</summary>
    internal static bool IsHarmless(Exception exception) => exception is OperationCanceledException;
}
