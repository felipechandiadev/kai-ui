"use client";

import { MultimediaSingleSlot } from "./MultimediaSingleSlot";
import type {
  MultimediaAvatarActionPlacement,
  MultimediaAvatarSize,
  MultimediaUpdaterVariant,
} from "./types";

export type MultimediaUpdaterProps = {
  currentUrl?: string | null;
  currentType?: "image" | "video" | string;
  onFileChange?: (file: File | null) => void;
  acceptedTypes?: string[];
  maxSize?: number;
  variant?: MultimediaUpdaterVariant;
  /** Solo `variant="avatar"`: sm | md | lg. Default md. */
  avatarSize?: MultimediaAvatarSize;
  /** Solo `variant="avatar"`: below (default) | edge (badge en borde). */
  actionPlacement?: MultimediaAvatarActionPlacement;
  allowDragDrop?: boolean;
  className?: string;
  disabled?: boolean;
};

/**
 * Updater de imagen única (banner / avatar / logo).
 * API alineada a realEstatePlatform-3 / BShop.
 */
export function MultimediaUpdater({
  currentUrl,
  currentType = "image",
  onFileChange,
  acceptedTypes = ["image/*"],
  maxSize = 8,
  variant = "default",
  avatarSize = "md",
  actionPlacement = "below",
  allowDragDrop = false,
  className = "",
  disabled = false,
}: MultimediaUpdaterProps) {
  const singleVariant =
    variant === "avatar" ? "avatar" : variant === "logo" ? "logo" : "banner";

  return (
    <div className={className} data-test-id="multimedia-updater">
      <MultimediaSingleSlot
        variant={singleVariant}
        currentUrl={currentUrl}
        currentType={currentType === "video" ? "video" : "image"}
        disabled={disabled}
        allowDragDrop={allowDragDrop}
        acceptedTypes={acceptedTypes}
        maxSizeMb={maxSize}
        avatarSize={avatarSize}
        actionPlacement={actionPlacement}
        onFileChange={onFileChange}
      />
    </div>
  );
}

export default MultimediaUpdater;
