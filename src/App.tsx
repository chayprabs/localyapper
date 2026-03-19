import { Overlay } from "@/components/overlay/Overlay";
import { SettingsLayout } from "@/components/settings/SettingsLayout";

export function App() {
  const isOverlay = window.location.pathname === "/overlay";

  if (isOverlay) {
    return <Overlay />;
  }

  return <SettingsLayout />;
}
