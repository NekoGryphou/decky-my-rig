using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using MyRigConfig.Core.Shared;

namespace MyRigConfig.Features.Host;

/// <param name="rename">The view model's rename command; the row passes its own ID.</param>
/// <param name="remove">The view model's remove command; the row passes its own ID.</param>
public sealed partial class ManagedDeviceSummary(PairedDeviceSummary device,
    IAsyncRelayCommand<Guid> rename, IAsyncRelayCommand<Guid> remove) : ObservableObject
{
    public IAsyncRelayCommand<Guid> RenameCommand => rename;
    public IAsyncRelayCommand<Guid> RemoveCommand => remove;
    public Guid DeviceId => device.DeviceId;
    public string Name => device.Name;
    public string RemoveLabel => HostMessages.RemoveLabel(Name);
    public string RenameLabel => HostMessages.RenameLabel(Name);
    public string RemoveAutomationId => $"{HostPageConstants.RemoveAutomationPrefix}{DeviceId:D}";
    public string RenameAutomationId => $"{HostPageConstants.RenameAutomationPrefix}{DeviceId:D}";
    public string MoreAutomationId => $"{HostPageConstants.MoreAutomationPrefix}{DeviceId:D}";
    public string MoreLabel => HostMessages.MoreOptionsLabel(Name);
    [ObservableProperty] public partial bool CanEdit { get; internal set; }
}
