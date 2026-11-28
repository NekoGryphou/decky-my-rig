using MyRigConfig.Shell;

namespace MyRigConfig.Tests;

[TestClass]
public sealed class ShellHelpersTests
{
    [TestMethod]
    public void TheIconIsFoundNextToTheAppNotInTheWorkingDirectory()
    {
        var baseDirectory = Path.Combine(Path.GetTempPath(), "myrig-app");
        Assert.AreEqual(Path.Combine(baseDirectory, "Assets", "AppIcon.ico"), ShellHelpers.IconPath(baseDirectory));
        Assert.IsTrue(Path.IsPathRooted(ShellHelpers.IconPath(baseDirectory)));
    }

    [TestMethod]
    public void ASecondLaunchDoesNotShowAWindowThatIsClosing()
    {
        Assert.IsTrue(ShellHelpers.ShouldBringToFront(windowClosing: false));
        Assert.IsFalse(ShellHelpers.ShouldBringToFront(windowClosing: true));
    }

    [TestMethod]
    public void OnlyACancellationIsSwallowedByTheAppHandler()
    {
        Assert.IsTrue(ShellHelpers.IsHarmless(new OperationCanceledException()));
        Assert.IsTrue(ShellHelpers.IsHarmless(new TaskCanceledException()));
        Assert.IsFalse(ShellHelpers.IsHarmless(new InvalidOperationException()));
        Assert.IsFalse(ShellHelpers.IsHarmless(new NullReferenceException()));
    }
}
