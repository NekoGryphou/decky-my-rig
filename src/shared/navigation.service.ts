import { Navigation } from "@decky/ui";
import { SETTINGS_EDITOR_ROUTE } from "./routes.consts";

export class EditorNavigation {
  private static pcId: string | null = null;
  private static listeners = new Set<() => void>();
  static getSnapshot = (): string | null => EditorNavigation.pcId;
  static subscribe = (listener: () => void): (() => void) => {
    EditorNavigation.listeners.add(listener);
    return () => {
      EditorNavigation.listeners.delete(listener);
    };
  };
  static reset(): void {
    this.select(null);
  }
  static open(id: string | null): void {
    this.select(id);
    Navigation.CloseSideMenus();
    Navigation.Navigate(SETTINGS_EDITOR_ROUTE);
  }
  private static select(id: string | null): void {
    if (this.pcId === id) return;
    this.pcId = id;
    for (const listener of this.listeners) listener();
  }
}
