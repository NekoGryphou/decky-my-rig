using MyRigConfig.Shared;
using MyRigConfig.Core.LocalControl;
using MyRigConfig.Features.Rename;

namespace MyRigConfig.Tests;

[TestClass]
public sealed class RenameDialogViewModelTests
{
    [TestMethod]
    public void SaveNeedsAValidDifferentTrimmedName()
    {
        var model = new RenameDialogViewModel("SteamDeck");
        Assert.IsFalse(model.CanSave);
        model.Name = "  Living room device  ";
        Assert.IsTrue(model.CanSave);
        Assert.AreEqual("Living room device", model.ChosenName);
        model.Name = " SteamDeck ";
        Assert.IsFalse(model.CanSave);
        Assert.AreEqual("", model.Error);
    }

    [TestMethod]
    public void InvalidNamesExplainWhy()
    {
        var model = new RenameDialogViewModel("SteamDeck");
        foreach (var name in new[] { "", new string('x', 81), "Device‮evil" })
        {
            model.Name = name;
            Assert.IsFalse(model.CanSave, name);
            Assert.AreEqual(SharedMessages.InvalidDeviceName, model.Error, name);
        }
    }

    [TestMethod]
    public void TheSharedNameFixtureIsCheckedHere()
    {
        // ! The service's tests read the same file, so both sides follow one name rule.
        using var fixture = System.Text.Json.JsonDocument.Parse(
            File.ReadAllBytes(Path.Combine(AppContext.BaseDirectory, "Fixtures", "names.json")));
        foreach (var name in fixture.RootElement.GetProperty("valid").EnumerateArray())
            Assert.IsTrue(LocalControlProtocolService.IsValidName(name.GetString()), name.GetString());
        foreach (var invalid in fixture.RootElement.GetProperty("invalid").EnumerateArray())
            Assert.IsFalse(LocalControlProtocolService.IsValidName(invalid.GetProperty("name").GetString()),
                invalid.GetProperty("why").GetString());
    }

    [TestMethod]
    public void NameRuleMatchesTheService()
    {
        Assert.IsTrue(LocalControlProtocolService.IsValidName("Salon – Steam Deck 🎮"));
        foreach (var name in new[] { " Device", "Device\n", "Device​", "⁦Device⁩", "﻿Device", "Device­" })
            Assert.IsFalse(LocalControlProtocolService.IsValidName(name), name);
        Assert.ThrowsException<System.Text.Json.JsonException>(() =>
            LocalControlProtocolService.RenameRequest(Guid.Empty, Guid.Empty, "Device‮"));
    }
}
