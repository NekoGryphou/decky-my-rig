using System.Buffers.Binary;
using System.Security;
using System.Text;
using System.Text.Json;
using System.Runtime.Versioning;
using System.IO.Pipes;
using System.Security.Principal;
using MyRigConfig.Core.Host;
using MyRigConfig.Core.LocalControl;

namespace MyRigConfig.Tests;

[TestClass]
[SupportedOSPlatform("windows10.0")]
public class HostControlClientTests
{
    [TestMethod]
    public void ProductionServiceAndPipeNamesMatchTheHost()
    {
        Assert.AreEqual("MyRigHost", HostConstants.ServiceName);
        Assert.AreEqual(@"\\.\pipe\MyRigHost.Control.v1", LocalControlConstants.PipePath);
    }

    [TestMethod]
    public void CallerCancellationIsReportedAsCancellationNotHostFailure()
    {
        using var cancelled = new CancellationTokenSource();
        cancelled.Cancel();
        foreach (var failure in new Exception[] { new HostControlException(true), new IOException(), new OperationCanceledException() })
        {
            Assert.IsInstanceOfType<OperationCanceledException>(
                HostControlClientService.Failure(failure, true, LocalControlOperation.SetPort, cancelled.Token));
        }
        var hostFailure = HostControlClientService.Failure(new IOException(), true, LocalControlOperation.SetPort, CancellationToken.None);
        Assert.IsTrue(hostFailure is HostControlException { OutcomeUncertain: true });
    }

    [TestMethod]
    public async Task PipeOwnedByCurrentUserIsRefusedAsTheService()
    {
        if (!OperatingSystem.IsWindows()) Assert.Inconclusive("Windows-only client fixture.");
        var name = "MyRigHost.Control.Test.App." + Guid.NewGuid().ToString("N");
        using var deadline = new CancellationTokenSource(TimeSpan.FromSeconds(3));
        using var server = new NamedPipeServerStream(name, PipeDirection.InOut, 1,
            PipeTransmissionMode.Byte, PipeOptions.Asynchronous | PipeOptions.CurrentUserOnly);
        var connected = server.WaitForConnectionAsync(deadline.Token);
        using (var client = new NamedPipeClientStream(".", name, PipeDirection.InOut,
            PipeOptions.Asynchronous, TokenImpersonationLevel.Identification))
        {
            await client.ConnectAsync(2000, deadline.Token);
            await connected;
            var expectedService = new SecurityIdentifier("S-1-5-80-1141499660-3876455147-362637617-4292076426-725672618");
            Assert.ThrowsException<SecurityException>(() => ControlServerIdentityNative.RequireOwner(client.SafePipeHandle, expectedService));
        }
        // - The refused client closed its handle without sending a frame prefix.
        var buffer = new byte[1];
        Assert.AreEqual(0, await server.ReadAsync(buffer, deadline.Token));
    }

    [TestMethod]
    public async Task PipeOwnerMatchingExpectedSidIsAccepted()
    {
        if (!OperatingSystem.IsWindows()) Assert.Inconclusive("Windows-only client fixture.");
        var name = "MyRigHost.Control.Test.Owner." + Guid.NewGuid().ToString("N");
        using var deadline = new CancellationTokenSource(TimeSpan.FromSeconds(3));
        using var server = new NamedPipeServerStream(name, PipeDirection.InOut, 1,
            PipeTransmissionMode.Byte, PipeOptions.Asynchronous | PipeOptions.CurrentUserOnly);
        var connected = server.WaitForConnectionAsync(deadline.Token);
        using var client = new NamedPipeClientStream(".", name, PipeDirection.InOut,
            PipeOptions.Asynchronous, TokenImpersonationLevel.Identification);
        await client.ConnectAsync(2000, deadline.Token);
        await connected;
        using var current = WindowsIdentity.GetCurrent();
        ControlServerIdentityNative.RequireOwner(client.SafePipeHandle, current.Owner!);
    }

    [TestMethod]
    public async Task ManagementClientFramesAndParsesAcrossTransportBoundary()
    {
        var response = TestHost.Bytes(TestHost.ListJson((TestHost.DeviceIdText, "Bedroom device")));
        var frame = new byte[4 + response.Length];
        BinaryPrimitives.WriteUInt32LittleEndian(frame, (uint)response.Length);
        response.CopyTo(frame, 4);
        using var pipe = new Duplex(frame);
        var checks = 0;
        var client = new HostControlClientService((request, sending, token) =>
            HostControlClientService.ExchangeFrameAsync(pipe, request, () => checks++, sending, token));
        var reply = await client.ListAsync(CancellationToken.None) as LocalControlListReply;
        Assert.IsNotNull(reply);
        Assert.AreEqual("Bedroom device", reply.PairedDevices.Single().Name);
        Assert.AreEqual(TestHost.Id, reply.HostId);
        Assert.AreEqual(2, checks);
        var written = pipe.Output.ToArray();
        Assert.AreEqual((uint)(written.Length - 4), BinaryPrimitives.ReadUInt32LittleEndian(written));
        using var requestBody = JsonDocument.Parse(written.AsMemory(4));
        Assert.AreEqual(2, requestBody.RootElement.EnumerateObject().Count());
        Assert.AreEqual(1, requestBody.RootElement.GetProperty("version").GetInt32());
        Assert.AreEqual("list", requestBody.RootElement.GetProperty("operation").GetString());
    }

    [TestMethod]
    [DataRow(typeof(IOException))]
    [DataRow(typeof(SecurityException))]
    [DataRow(typeof(UnauthorizedAccessException))]
    [DataRow(typeof(System.ComponentModel.Win32Exception))]
    [DataRow(typeof(TimeoutException))]
    [DataRow(typeof(IdentityNotMappedException))]
    public async Task TransportFailuresAreHostFailures(Type failure)
    {
        var client = new HostControlClientService((_, _, _) => throw (Exception)Activator.CreateInstance(failure)!);
        await Assert.ThrowsExceptionAsync<HostControlException>(() => client.ListAsync(CancellationToken.None));
    }

    [TestMethod]
    [DataRow(typeof(InvalidOperationException))]
    [DataRow(typeof(NullReferenceException))]
    [DataRow(typeof(ArgumentException))]
    public async Task ProgrammingErrorsAreNotReportedAsAnUnavailableHost(Type failure)
    {
        var client = new HostControlClientService((_, _, _) => throw (Exception)Activator.CreateInstance(failure)!);
        Exception? thrown = null;
        try { await client.ListAsync(CancellationToken.None); }
        catch (Exception exception) { thrown = exception; }
        Assert.IsInstanceOfType(thrown, failure);
    }

    [TestMethod]
    public async Task InvalidTransportReplyStillFailsClosedWithoutRetry()
    {
        var calls = 0;
        var client = new HostControlClientService((_, sending, _) =>
        {
            calls++;
            sending();
            return Task.FromResult<byte[]>([123]);
        });
        await Assert.ThrowsExceptionAsync<HostControlException>(() => client.ListAsync(CancellationToken.None));
        Assert.AreEqual(1, calls);
    }

    private sealed class Duplex(byte[] response) : Stream
    {
        private readonly MemoryStream input = new(response);
        internal readonly MemoryStream Output = new();
        internal long UnreadBytes => input.Length - input.Position;
        public override bool CanRead => true;
        public override bool CanWrite => true;
        public override bool CanSeek => false;
        public override long Length => throw new NotSupportedException();
        public override long Position { get => throw new NotSupportedException(); set => throw new NotSupportedException(); }
        public override void Flush() { }
        public override int Read(byte[] buffer, int offset, int count) => input.Read(buffer, offset, Math.Min(count, 1));
        public override void Write(byte[] buffer, int offset, int count) => Output.Write(buffer, offset, count);
        public override ValueTask<int> ReadAsync(Memory<byte> buffer, CancellationToken token = default) =>
            input.ReadAsync(buffer[..Math.Min(buffer.Length, 1)], token);
        public override ValueTask WriteAsync(ReadOnlyMemory<byte> buffer, CancellationToken token = default) =>
            Output.WriteAsync(buffer, token);
        public override long Seek(long offset, SeekOrigin origin) => throw new NotSupportedException();
        public override void SetLength(long value) => throw new NotSupportedException();
        protected override void Dispose(bool disposing)
        { if (disposing) { input.Dispose(); Output.Dispose(); } base.Dispose(disposing); }
    }

    [TestMethod]
    public async Task FailedIdentityVerificationWritesNoPrefixOrRequest()
    {
        if (!OperatingSystem.IsWindows()) Assert.Inconclusive("Windows-only client fixture.");
        using var pipe = new Duplex([]);
        var sending = false;
        await Assert.ThrowsExceptionAsync<SecurityException>(() => HostControlClientService.ExchangeFrameAsync(
            pipe, [1], () => throw new SecurityException("fixture denial"), () => sending = true, CancellationToken.None));
        Assert.AreEqual(0L, pipe.Output.Length);
        Assert.IsFalse(sending);
    }

    [TestMethod]
    public async Task ChangedServerAfterReplyIsRejectedBeforeReturning()
    {
        if (!OperatingSystem.IsWindows()) Assert.Inconclusive("Windows-only client fixture.");
        using var pipe = new Duplex([1, 0, 0, 0, 42]);
        var checks = 0;
        await Assert.ThrowsExceptionAsync<SecurityException>(() => HostControlClientService.ExchangeFrameAsync(
            pipe, [7], () =>
            {
                if (++checks != 2) return;
                Assert.AreEqual(0L, pipe.UnreadBytes, "Recheck must follow the complete reply.");
                throw new SecurityException("Fixture server restarted.");
            }, () => { }, CancellationToken.None));
        Assert.AreEqual(2, checks);
        Assert.AreEqual(5L, pipe.Output.Length);
    }

    [TestMethod]
    public async Task OneVerifiedFrameSupportsPartialReads()
    {
        if (!OperatingSystem.IsWindows()) Assert.Inconclusive("Windows-only client fixture.");
        using var pipe = new Duplex([3, 0, 0, 0, 10, 11, 12]);
        var verified = false;
        var sending = false;
        var reply = await HostControlClientService.ExchangeFrameAsync(pipe, [7, 8], () => verified = true,
            () => { Assert.IsTrue(verified); sending = true; }, CancellationToken.None);
        CollectionAssert.AreEqual(new byte[] { 10, 11, 12 }, reply);
        CollectionAssert.AreEqual(new byte[] { 2, 0, 0, 0, 7, 8 }, pipe.Output.ToArray());
        Assert.IsTrue(sending);
    }

    [TestMethod]
    public async Task RefusesResponseBoundsAndTruncatedFramesWithoutRetry()
    {
        if (!OperatingSystem.IsWindows()) Assert.Inconclusive("Windows-only client fixture.");
        foreach (var length in new uint[] { 0, 8193, uint.MaxValue })
        {
            var prefix = new byte[4];
            BinaryPrimitives.WriteUInt32LittleEndian(prefix, length);
            using var pipe = new Duplex(prefix);
            await Assert.ThrowsExceptionAsync<IOException>(() => HostControlClientService.ExchangeFrameAsync(
                pipe, [7], () => { }, () => { }, CancellationToken.None));
            Assert.AreEqual(5L, pipe.Output.Length);
        }
        using var truncated = new Duplex([2, 0, 0, 0]);
        await Assert.ThrowsExceptionAsync<EndOfStreamException>(() => HostControlClientService.ExchangeFrameAsync(
            truncated, [7], () => { }, () => { }, CancellationToken.None));
        Assert.AreEqual(5L, truncated.Output.Length);
    }

    [TestMethod]
    public async Task PreCancelledAndInvalidRequestsWriteNothing()
    {
        if (!OperatingSystem.IsWindows()) Assert.Inconclusive("Windows-only client fixture.");
        using var pipe = new Duplex([]);
        await Assert.ThrowsExceptionAsync<OperationCanceledException>(() => HostControlClientService.ExchangeFrameAsync(
            pipe, [7], () => Assert.Fail(), () => Assert.Fail(), new CancellationToken(true)));
        foreach (var request in new byte[][] { [], new byte[1025] })
            await Assert.ThrowsExceptionAsync<HostControlException>(() => HostControlClientService.ExchangeFrameAsync(
                pipe, request, () => Assert.Fail(), () => Assert.Fail(), CancellationToken.None));
        Assert.AreEqual(0L, pipe.Output.Length);
    }
}
