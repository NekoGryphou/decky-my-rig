export const testShutdownsReceived = (count: number) =>
  count === 1 ? "Test shutdown received." : `${count} test shutdowns received.`;
