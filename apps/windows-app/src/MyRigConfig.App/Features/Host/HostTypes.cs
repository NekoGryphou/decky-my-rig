using MyRigConfig.Shared;

namespace MyRigConfig.Features.Host;

public sealed record HostNotice(string Title, string Message, MessageTone Severity);

/// <summary>The dialogs the page shows for the view model's commands. Each returns the user's
/// answer.</summary>
public interface IHostDialogs
{
    Task ShowPairingAsync(CancellationToken cancellationToken);
    Task<bool> ConfirmRemovalAsync(string name);
    /// <returns>The new name, or null when the user cancels.</returns>
    Task<string?> AskNameAsync(string current);
    /// <returns>The chosen port, or null when the user cancels.</returns>
    Task<int?> AskPortAsync(int current);
}
