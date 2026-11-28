using System.Text;
using MyRigConfig.Core.Host;
using MyRigConfig.Core.LocalControl;
using MyRigConfig.Features.Host;

namespace MyRigConfig.Tests;

/// <summary>The host and device the fixtures talk about, and the helpers that build a view model
/// around a fake service.</summary>
internal static class TestHost
{
    internal const string IdText = "03c85317-9d93-4fb3-a04b-91a0e16668c6";
    internal const string DeviceIdText = "11111111-1111-4111-8111-111111111111";
    internal static readonly Guid Id = Guid.Parse(IdText);
    internal static readonly Guid DeviceId = Guid.Parse(DeviceIdText);

    internal static string Envelope(string? hostId = null) =>
        $"\"version\":1,\"ok\":true,\"host_id\":\"{hostId ?? IdText}\"";

    /// <summary>A list reply; each device is an ID and a name.</summary>
    internal static string ListJson(params (string Id, string Name)[] devices) =>
        $"{{{Envelope()},\"paired_devices\":[{string.Join(',', devices.Select(d => $"{{\"device_id\":\"{d.Id}\",\"name\":\"{d.Name}\"}}"))}]}}";

    internal static byte[] Bytes(string json) => Encoding.UTF8.GetBytes(json);

    internal static HostStatus Status(int port = 47991, bool testMode = false) => new(port, testMode, "listening");

    /// <summary>Starting or restarting the real service must never happen in a test.</summary>
    internal static Task NoServiceControl(CancellationToken cancellationToken) =>
        throw new AssertFailedException("The service must not be started or restarted by this test");

    internal static Task<HostObservation> Running(CancellationToken cancellationToken) =>
        Task.FromResult(new HostObservation(ServiceState.Running, Status(), null));

    internal static HostViewModel Model(HostControlClientService client,
        Func<CancellationToken, Task<HostObservation>>? diagnostics = null, TimeProvider? clock = null,
        Func<CancellationToken, Task>? startService = null, Func<CancellationToken, Task>? restartService = null,
        IPortProbe? probe = null) =>
        new(client, diagnostics ?? Running, clock ?? TimeProvider.System,
            startService ?? NoServiceControl, restartService ?? NoServiceControl,
            probe ?? new StubPortProbe(_ => null));

    /// <summary>A client whose service answers every request with <paramref name="reply"/>; a null
    /// reply means an empty list.</summary>
    internal static HostControlClientService Client(Func<string, string?> reply) =>
        new((request, sending, _) =>
        {
            sending();
            return Task.FromResult(Bytes(reply(Encoding.UTF8.GetString(request)) ?? ListJson()));
        });
}

/// <summary>A clock that only moves when the test says so.</summary>
internal sealed class FakeClock : TimeProvider
{
    private long ticks;
    public override long TimestampFrequency => TimeSpan.TicksPerSecond;
    public override long GetTimestamp() => ticks;
    internal void Advance(TimeSpan duration) => ticks += duration.Ticks;
}

/// <summary>A port probe whose answers the test writes as lambdas, so no test reads the real TCP
/// table.</summary>
internal sealed class StubPortProbe(Func<int, int?> findListener, Func<int, string?>? processName = null) : IPortProbe
{
    private readonly List<int> queries = [];
    internal int[] Queries { get { lock (queries) return [.. queries]; } }

    public int? FindListener(int port, int? excludedProcessId)
    {
        lock (queries) queries.Add(port);
        return findListener(port) is { } owner && owner != excludedProcessId ? owner : null;
    }

    public string? ProcessName(int processId) => processName?.Invoke(processId);
}
