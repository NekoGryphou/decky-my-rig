using System.Text;
using System.Text.Json;
using MyRigConfig.Core.LocalControl;
using MyRigConfig.Core.Management;
using MyRigConfig.Core.Shared;
using MyRigConfig.Features.Port;
using MyRigConfig.Shared;

namespace MyRigConfig.Tests;

[TestClass]
public sealed class LocalControlProtocolTests
{
    [TestMethod]
    public void WireRepliesHaveSourceGeneratedContracts()
    {
        Assert.IsNotNull(LocalControlJsonContext.Default.ListReply);
        Assert.IsNotNull(LocalControlJsonContext.Default.PairingReply);
    }

    [TestMethod]
    public void NameAndPortLimitsAreOneSourceForRulesAndMessages()
    {
        Assert.IsTrue(LocalControlProtocolService.IsValidName(new string('a', LocalControlConstants.MaxNameLength)));
        Assert.IsFalse(LocalControlProtocolService.IsValidName(new string('a', LocalControlConstants.MaxNameLength + 1)));
        Assert.IsTrue(SharedMessages.InvalidDeviceName.Contains($"1 to {LocalControlConstants.MaxNameLength} ", StringComparison.Ordinal));
        Assert.IsTrue(PortMessages.OutOfRange.Contains($"{LocalControlConstants.MinSettablePort} to {LocalControlConstants.MaxPort}", StringComparison.Ordinal));
        Assert.IsTrue(LocalControlProtocolService.IsSettablePort(LocalControlConstants.MinSettablePort));
        Assert.IsFalse(LocalControlProtocolService.IsSettablePort(LocalControlConstants.MinSettablePort - 1));
        Assert.IsTrue(LocalControlProtocolService.IsSettablePort(LocalControlConstants.MaxPort));
        Assert.IsFalse(LocalControlProtocolService.IsSettablePort(LocalControlConstants.MaxPort + 1));
    }

    [TestMethod]
    public void NoPairingReplyPrintsItsCode()
    {
        const string code = "12345678";
        Assert.IsFalse(new PairingReply(1, true, Host, code, 120).ToString().Contains(code, StringComparison.Ordinal));
        Assert.IsFalse(new LocalControlPairingReply(HostId, code).ToString().Contains(code, StringComparison.Ordinal));
    }

    private const string Host = TestHost.IdText;
    private const string Device = TestHost.DeviceIdText;
    private static readonly Guid HostId = Guid.Parse(Host);
    private static byte[] Bytes(string json) => Encoding.UTF8.GetBytes(json);
    private static string Envelope(string payload) => $"{{\"version\":1,\"ok\":true,\"host_id\":\"{Host}\",{payload}}}";
    private static string Summary(string name = "Steam Deck") => $"{{\"device_id\":\"{Device}\",\"name\":{JsonSerializer.Serialize(name)}}}";
    private static void Reject(string json, LocalControlOperation operation = LocalControlOperation.List) =>
        Assert.ThrowsException<JsonException>(() => LocalControlProtocolService.ParseReply(Bytes(json), operation));
    private static byte[] Fixture(string name) => File.ReadAllBytes(Path.Combine(AppContext.BaseDirectory, "Fixtures", name));

    [TestMethod]
    public void SharedFixturesMatchRequestsAndReplies()
    {
        CollectionAssert.AreEqual(Fixture("list-request.json").AsSpan().TrimEnd((byte)'\n').ToArray(), LocalControlProtocolService.ListRequest());
        CollectionAssert.AreEqual(Fixture("remove-request.json").AsSpan().TrimEnd((byte)'\n').ToArray(), LocalControlProtocolService.RemoveRequest(HostId, Guid.Parse(Device)));
        CollectionAssert.AreEqual(Fixture("open-pairing-request.json").AsSpan().TrimEnd((byte)'\n').ToArray(), LocalControlProtocolService.OpenPairingRequest(HostId, "Device"));
        CollectionAssert.AreEqual(Fixture("set-port-request.json").AsSpan().TrimEnd((byte)'\n').ToArray(), LocalControlProtocolService.SetPortRequest(HostId, 48000));
        CollectionAssert.AreEqual(Fixture("status-request.json").AsSpan().TrimEnd((byte)'\n').ToArray(), LocalControlProtocolService.StatusRequest());
        CollectionAssert.AreEqual(Fixture("set-test-mode-request.json").AsSpan().TrimEnd((byte)'\n').ToArray(), LocalControlProtocolService.SetTestModeRequest(HostId, false));

        var list = (LocalControlListReply)LocalControlProtocolService.ParseReply(Fixture("list-reply.json"), LocalControlOperation.List, HostId);
        Assert.AreEqual(2, list.PairedDevices.Count);
        Assert.AreEqual("Other device", list.PairedDevices[1].Name);
        var removed = (LocalControlRemoveReply)LocalControlProtocolService.ParseReply(Fixture("remove-reply.json"), LocalControlOperation.Remove, HostId);
        Assert.IsTrue(removed.Removed);
        var pairing = (LocalControlPairingReply)LocalControlProtocolService.ParseReply(Fixture("open-pairing-reply.json"), LocalControlOperation.OpenPairing, HostId);
        Assert.AreEqual("00123456", pairing.Code);
        var port = (LocalControlPortReply)LocalControlProtocolService.ParseReply(Fixture("set-port-reply.json"), LocalControlOperation.SetPort, HostId);
        Assert.AreEqual(48000, port.Port);
        var status = (LocalControlStatusReply)LocalControlProtocolService.ParseReply(Fixture("status-reply.json"), LocalControlOperation.Status, HostId);
        Assert.AreEqual(49000, status.Port);
        Assert.IsTrue(status.TestMode);
        Assert.AreEqual("0.0.0", status.ServiceVersion);
        Assert.AreEqual("unavailable", status.ListenerState);
        var testMode = (LocalControlTestModeReply)LocalControlProtocolService.ParseReply(Fixture("set-test-mode-reply.json"), LocalControlOperation.SetTestMode, HostId);
        Assert.IsFalse(testMode.TestMode);
        var error = (LocalControlErrorReply)LocalControlProtocolService.ParseReply(Fixture("error-reply.json"), LocalControlOperation.List);
        Assert.AreEqual("invalid_request", error.Error);
        Assert.ThrowsException<JsonException>(() => LocalControlProtocolService.ParseReply(Fixture("invalid-reply-code.json"), LocalControlOperation.OpenPairing, HostId));
    }

    [TestMethod]
    public void SetTestModeRejectsMalformedReplies()
    {
        foreach (var payload in new[] { "\"test_mode\":0", "\"test_mode\":\"false\"", "\"test_mode\":false,\"extra\":1", "\"port\":49000" })
            Reject(Envelope(payload), LocalControlOperation.SetTestMode);
        foreach (var payload in new[] { "\"port\":0,\"test_mode\":false,\"service_version\":\"0.1.0\"", "\"port\":49000,\"test_mode\":0,\"service_version\":\"0.1.0\"", "\"port\":49000,\"test_mode\":false,\"service_version\":\"\"" })
            Reject(Envelope(payload), LocalControlOperation.Status);
    }

    [TestMethod]
    public void SetPortAcceptsOnlyTheSettableRange()
    {
        foreach (var port in new[] { 0, 80, 1023, 65536, -1 })
            Assert.ThrowsException<JsonException>(() => LocalControlProtocolService.SetPortRequest(HostId, port));
        Assert.IsTrue(LocalControlProtocolService.IsSettablePort(1024));
        Assert.IsTrue(LocalControlProtocolService.IsSettablePort(65535));
        foreach (var value in new[] { "1023", "65536", "48000.5", "\"48000\"", "true" })
            Reject(Envelope($"\"port\":{value}"), LocalControlOperation.SetPort);
        Reject(Envelope("\"port\":48000,\"extra\":1"), LocalControlOperation.SetPort);
        Reject(Envelope("\"removed\":true"), LocalControlOperation.SetPort);
    }

    [TestMethod]
    public void RequestsContainOnlyRequiredFieldsAndCanonicalIds()
    {
        Assert.AreEqual("{\"version\":1,\"operation\":\"list\"}", Encoding.UTF8.GetString(LocalControlProtocolService.ListRequest()));
        Assert.AreEqual($"{{\"version\":1,\"operation\":\"remove\",\"host_id\":\"{Host}\",\"device_id\":\"{Device}\"}}",
            Encoding.UTF8.GetString(LocalControlProtocolService.RemoveRequest(HostId, Guid.Parse(Device))));
        using var request = JsonDocument.Parse(LocalControlProtocolService.OpenPairingRequest(HostId, "Steam Deck"));
        CollectionAssert.AreEquivalent(new[] { "version", "operation", "host_id", "name" }, request.RootElement.EnumerateObject().Select(p => p.Name).ToArray());
        Assert.AreEqual("open_pairing", request.RootElement.GetProperty("operation").GetString());
        Assert.AreEqual("Steam Deck", request.RootElement.GetProperty("name").GetString());
        Assert.IsTrue(LocalControlProtocolService.OpenPairingRequest(HostId, string.Concat(Enumerable.Repeat("😀", 80))).Length <= 1024);
        const string special = "Device \"\\ 😀";
        using var escaped = JsonDocument.Parse(LocalControlProtocolService.OpenPairingRequest(HostId, special));
        Assert.AreEqual(special, escaped.RootElement.GetProperty("name").GetString());
    }

    [TestMethod]
    public void ParseSuccessEnvelopesAndSafeErrors()
    {
        var list = (LocalControlListReply)LocalControlProtocolService.ParseReply(Bytes(Envelope($"\"paired_devices\":[{Summary()}]")), LocalControlOperation.List, HostId);
        Assert.AreEqual(HostId, list.HostId);
        Assert.AreEqual(Guid.Parse(Device), list.PairedDevices[0].DeviceId);
        Assert.AreEqual("Steam Deck", list.PairedDevices[0].Name);
        foreach (var removed in new[] { "true", "false" })
        {
            var reply = (LocalControlRemoveReply)LocalControlProtocolService.ParseReply(Bytes(Envelope($"\"removed\":{removed}")), LocalControlOperation.Remove, HostId);
            Assert.AreEqual(removed == "true", reply.Removed);
        }
        var code = (LocalControlPairingReply)LocalControlProtocolService.ParseReply(Bytes(Envelope("\"code\":\"00123456\",\"expires_in_seconds\":120")), LocalControlOperation.OpenPairing, HostId);
        Assert.AreEqual("00123456", code.Code);
        Assert.IsFalse(code.ToString().Contains(code.Code));
        foreach (var error in new[] { "unauthorized", "invalid_request", "wrong_host", "storage_failed", "storage_conflict", "recovery_required", "unavailable", "control_failed" })
        {
            var reply = (LocalControlErrorReply)LocalControlProtocolService.ParseReply(Bytes($"{{\"version\":1,\"ok\":false,\"error\":\"{error}\"}}"), LocalControlOperation.List);
            Assert.AreEqual(error, reply.Error);
        }
    }

    [TestMethod]
    public void RejectMalformedBoundsUtf8DepthAndEnvelopeFields()
    {
        foreach (var json in new[] { "", "[]", "null", "{}", "{", "{} {}",
            "{\"version\":1,\"ok\":false,\"error\":\"C:/secret\"}",
            "{\"version\":1,\"ok\":false,\"error\":\"unavailable\",\"token\":\"secret\"}",
            "{\"version\":1,\"version\":1,\"ok\":false,\"error\":\"unavailable\"}",
            "{\"version\":1.0,\"ok\":false,\"error\":\"unavailable\"}",
            "{\"version\":\"1\",\"ok\":false,\"error\":\"unavailable\"}",
            "{\"version\":1,\"ok\":0,\"error\":\"unavailable\"}",
            "{\"version\":1,\"ok\":false,\"error\":0}", "[[[[[[]]]]]]" }) Reject(json);
        Assert.ThrowsException<JsonException>(() => LocalControlProtocolService.ParseReply(new byte[8193], LocalControlOperation.List));
        Assert.ThrowsException<JsonException>(() => LocalControlProtocolService.ParseReply(new byte[] { 0xff }, LocalControlOperation.List));
        Reject(Envelope("\"paired_devices\":[]").Replace(Host, Host.ToUpperInvariant()));
        Assert.ThrowsException<JsonException>(() => LocalControlProtocolService.ParseReply(Bytes(Envelope("\"paired_devices\":[]")), LocalControlOperation.List, Guid.Empty));
        Reject(Envelope("\"paired_devices\":[],\"sid\":\"S-1-5-18\""));
        Reject(Envelope("\"paired_devices\":[],\"paired_devices\":[]"));
        Reject(Envelope("\"paired_devices\":{}"));
        Reject(Envelope("\"paired_devices\":[null]"));
        Reject(Envelope("\"removed\":true"));
        Reject(Envelope("\"removed\":1"), LocalControlOperation.Remove);
    }

    [TestMethod]
    public void RejectInvalidSummariesNamesAndDuplicateIdentities()
    {
        foreach (var name in new[] { "", " Device", "Device ", "\nDevice", "A\u007fB", new string('a', 81) })
        {
            Reject(Envelope($"\"paired_devices\":[{Summary(name)}]"));
            Assert.ThrowsException<JsonException>(() => LocalControlProtocolService.OpenPairingRequest(HostId, name));
        }
        Assert.ThrowsException<JsonException>(() => LocalControlProtocolService.OpenPairingRequest(HostId, "\ud800"));
        var summary = Summary();
        foreach (var invalid in new[] { summary.Replace("\"name\":", "\"token\":\"secret\",\"name\":"),
            summary.Replace(Device, "{" + Device + "}"), summary.Replace("\"name\":", "\"name\":\"Other\",\"name\":"),
            summary.Replace("\"Steam Deck\"", "null"), summary.Replace("\"Steam Deck\"", "\"\\ud800\"") })
            Reject(Envelope($"\"paired_devices\":[{invalid}]"));
        Reject(Envelope($"\"paired_devices\":[{summary},{summary}]"));
        Reject(Envelope("\"paired_devices\":[" + string.Join(",", Enumerable.Repeat(summary, 17)) + "]"));
        var valid = LocalControlProtocolService.ParseReply(Bytes(Envelope($"\"paired_devices\":[{Summary(string.Concat(Enumerable.Repeat("😀", 80)))}]")), LocalControlOperation.List);
        Assert.IsInstanceOfType<LocalControlListReply>(valid);
        var sixteen = Enumerable.Range(1, 16).Select(n => Summary().Replace(Device, $"00000000-0000-4000-8000-{n:000000000000}"));
        var full = (LocalControlListReply)LocalControlProtocolService.ParseReply(Bytes(Envelope("\"paired_devices\":[" + string.Join(",", sixteen) + "]")), LocalControlOperation.List);
        Assert.AreEqual(16, full.PairedDevices.Count);
    }

    [TestMethod]
    public void PairingCodeAndLifetimeAreExactAndNotListed()
    {
        foreach (var code in new[] { "1234567", "123456789", "１２３４５６７８", "1234567x" })
            Reject(Envelope($"\"code\":{JsonSerializer.Serialize(code)},\"expires_in_seconds\":120"), LocalControlOperation.OpenPairing);
        foreach (var lifetime in new[] { "119", "121", "120.0", "\"120\"", "true" })
            Reject(Envelope($"\"code\":\"12345678\",\"expires_in_seconds\":{lifetime}"), LocalControlOperation.OpenPairing);
        Reject(Envelope("\"code\":12345678,\"expires_in_seconds\":120"), LocalControlOperation.OpenPairing);
        Reject(Envelope("\"code\":\"12345678\",\"expires_in_seconds\":120,\"nonce\":\"secret\""), LocalControlOperation.OpenPairing);
        Reject(Envelope("\"code\":\"12345678\",\"expires_in_seconds\":120"));
    }
    [TestMethod]
    public void ClosePairingAcknowledgementIsStrictAndHostBound()
    {
        var operation = LocalControlOperation.ClosePairing;
        var valid = Bytes($"{{\"version\":1,\"ok\":true,\"host_id\":\"{Host}\"}}");
        Assert.IsNotNull(LocalControlProtocolService.ParseReply(valid, operation, HostId));
        Assert.ThrowsException<JsonException>(() => LocalControlProtocolService.ParseReply(valid, operation, Guid.Empty));
        foreach (var invalid in new[] {
            Encoding.UTF8.GetString(valid).Replace("\"version\":1", "\"version\":2"),
            Encoding.UTF8.GetString(valid).Replace("\"ok\":true", "\"ok\":true,\"ok\":true"),
            Encoding.UTF8.GetString(valid).Replace("}", ",\"code\":\"12345678\"}"),
            Encoding.UTF8.GetString(valid).Replace(Host, Host.ToUpperInvariant()),
        }) Reject(invalid, operation);
    }

    [TestMethod]
    public void RenameRequestAndReplyMatchTheServiceContract()
    {
        Assert.AreEqual(
            $"{{\"version\":1,\"operation\":\"rename\",\"host_id\":\"{Host}\",\"device_id\":\"{Device}\",\"name\":\"Living room device\"}}",
            Encoding.UTF8.GetString(LocalControlProtocolService.RenameRequest(HostId, Guid.Parse(Device), "Living room device")));
        var renamed = (LocalControlRenameReply)LocalControlProtocolService.ParseReply(
            Bytes(Envelope("\"renamed\":true")), LocalControlOperation.Rename, HostId);
        Assert.IsTrue(renamed.Renamed);
        Reject(Envelope("\"renamed\":1"), LocalControlOperation.Rename);
    }
}
