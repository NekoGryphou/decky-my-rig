const
  AppUninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{{#AppGuid}}_is1';
  OwnershipKey = 'Software\MyRig';
  DataOwnedValue = 'DataOwned';
  ManagerGroupOwnedValue = 'ManagerGroupOwned';
  // ? Managers are recorded by SID. The service grants them access directly, so it does not depend
  // ? on a group membership taking effect at sign-in.
  ManagerUsersValue = 'ManagerUsers';
  ServiceName = 'MyRigHost';
  ServiceKey = 'SYSTEM\CurrentControlSet\Services\MyRigHost';
  ServiceAccount = 'NT SERVICE\MyRigHost';
  ManagerGroup = 'My Rig Managers';
  FirewallRuleName = 'MyRig';
  // ? NET_FW_PROFILE2_ALL: the rule covers every profile, because a home network can be on Public
  // ? (the Windows default) or misclassified (mirrored WSL). It stays limited to this program and
  // ? the local subnet.
  FirewallAllProfiles = $7FFFFFFF;
  OwnerMarkerFile = 'installer-owner.txt';
  OwnerMarker = 'MyRig-owned-v1';
  NotifierRunValue = 'My Rig Notifier';
  NotifierIdentityKey = '\Software\Classes\AppUserModelId\MyRig.Notifier';
  FirstPort = '47991';
  ServiceStateTimeoutMs = 20000;
