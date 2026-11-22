using System.Collections.Concurrent;

namespace MyRigConfig.Tests;

internal sealed class PortUiContext : SynchronizationContext, IDisposable
{
    private readonly BlockingCollection<Action> work = new();
    private readonly Thread thread;
    private readonly ConcurrentBag<Task> operations = [];

    internal PortUiContext()
    {
        thread = new Thread(() =>
        {
            SetSynchronizationContext(this);
            foreach (var action in work.GetConsumingEnumerable()) action();
        }) { IsBackground = true, Name = "myrig-port-test-ui" };
        thread.Start();
    }

    public override void Post(SendOrPostCallback callback, object? state) => work.Add(() => callback(state));

    internal Task InvokeAsync(Action action) => InvokeAsync(() => { action(); return Task.CompletedTask; });

    internal Task InvokeAsync(Func<Task> action)
    {
        var complete = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        Post(_ => operations.Add(CompleteAsync(action, complete)), null);
        return complete.Task;
    }

    private static async Task CompleteAsync(Func<Task> action, TaskCompletionSource complete)
    {
        try { await action(); complete.SetResult(); }
        catch (Exception exception) { complete.SetException(exception); }
    }

    public void Dispose()
    {
        Task.WhenAll(operations).GetAwaiter().GetResult();
        work.CompleteAdding();
        thread.Join();
        work.Dispose();
    }
}

internal sealed class BlockingPortLookup : IDisposable
{
    private readonly Lock sync = new();
    private readonly ManualResetEventSlim release = new();
    private readonly List<int> queries = [];
    private int active;
    private int maximum;
    internal TaskCompletionSource Entered { get; } = new(TaskCreationOptions.RunContinuationsAsynchronously);
    internal int Maximum { get { lock (sync) return maximum; } }
    internal int[] Queries { get { lock (sync) return queries.ToArray(); } }
    internal int? Find(int port)
    {
        lock (sync) { queries.Add(port); maximum = Math.Max(maximum, ++active); }
        try
        {
            if (port == 48000) { Entered.TrySetResult(); release.Wait(); }
            return null;
        }
        finally { lock (sync) { active--; } }
    }
    internal void Release() => release.Set();
    public void Dispose() { release.Set(); release.Dispose(); }
}
