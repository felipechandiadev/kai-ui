"use client";

import { useEffect, useId, useRef, useState } from "react";
import { Image as ImageIcon, User } from "lucide-react";
import IconButton from "../IconButton/IconButton";
import Alert from "../Alert/Alert";
import type {
  MultimediaAvatarActionPlacement,
  MultimediaAvatarSize,
  MultimediaSingleVariant,
} from "./types";
import "./multimedia-single-slot.css";

export type { MultimediaAvatarActionPlacement, MultimediaAvatarSize };

const AVATAR_PLACEHOLDER_ICON: Record<MultimediaAvatarSize, number> = {
  sm: 28,
  md: 40,
  lg: 48,
};

const AVATAR_EDGE_BUTTON_SIZE: Record<MultimediaAvatarSize, "xs" | "sm"> = {
  sm: "xs",
  md: "xs",
  lg: "sm",
};

export type MultimediaSingleSlotProps = {
  variant: MultimediaSingleVariant;
  currentUrl?: string | null;
  currentType?: "image" | "video";
  disabled?: boolean;
  allowDragDrop?: boolean;
  acceptedTypes?: string[];
  maxSizeMb?: number;
  /** Solo variant=avatar: tamaño del círculo. Default md. */
  avatarSize?: MultimediaAvatarSize;
  /** Solo variant=avatar: botón debajo (default) o badge en el borde. */
  actionPlacement?: MultimediaAvatarActionPlacement;
  onFileChange?: (file: File | null) => void;
  className?: string;
  "data-test-id"?: string;
};

export function MultimediaSingleSlot({
  variant,
  currentUrl,
  currentType = "image",
  disabled = false,
  allowDragDrop = true,
  acceptedTypes = ["image/*", "video/*"],
  maxSizeMb = 9,
  avatarSize = "md",
  actionPlacement = "below",
  onFileChange,
  className = "",
  "data-test-id": testId = "multimedia-single-slot",
}: MultimediaSingleSlotProps) {
  const inputId = useId();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(
    currentUrl ?? null,
  );
  const [error, setError] = useState<string | null>(null);
  const [isDragOver, setIsDragOver] = useState(false);

  useEffect(() => {
    setPreviewUrl(currentUrl ?? null);
  }, [currentUrl]);

  const hasMedia = Boolean(previewUrl?.trim());
  const isAvatarEdge = variant === "avatar" && actionPlacement === "edge";

  const openFile = () => {
    if (!disabled) fileInputRef.current?.click();
  };

  const validate = (file: File): boolean => {
    if (!acceptedTypes.some((t) => file.type.match(t.replace("*", ".*")))) {
      setError(`Tipo no permitido: ${acceptedTypes.join(", ")}`);
      return false;
    }
    if (file.size > maxSizeMb * 1024 * 1024) {
      setError(`Máximo ${maxSizeMb}MB`);
      return false;
    }
    setError(null);
    return true;
  };

  const handleFile = (file: File) => {
    if (!validate(file)) return;
    const url = URL.createObjectURL(file);
    setPreviewUrl(url);
    onFileChange?.(file);
  };

  const onInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) handleFile(file);
    if (fileInputRef.current) fileInputRef.current.value = "";
  };

  const onDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(false);
    if (disabled) return;
    const file = e.dataTransfer.files?.[0];
    if (file) handleFile(file);
  };

  const onActionClick = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.stopPropagation();
    openFile();
  };

  const mediaClass =
    variant === "avatar"
      ? "fs-mm-slot__img fs-mm-slot__img--avatar"
      : variant === "logo"
        ? "fs-mm-slot__img fs-mm-slot__img--rounded fs-mm-slot__img--contain"
        : "fs-mm-slot__img fs-mm-slot__img--rounded";

  const renderPreview = () => {
    if (!previewUrl) {
      if (variant === "avatar") {
        return (
          <User
            className="fs-mm-slot__placeholder"
            size={AVATAR_PLACEHOLDER_ICON[avatarSize]}
          />
        );
      }
      return <ImageIcon className="fs-mm-slot__placeholder" size={48} />;
    }
    if (currentType === "video") {
      return <video src={previewUrl} className={mediaClass} controls muted />;
    }
    return (
      // eslint-disable-next-line @next/next/no-img-element
      <img src={previewUrl} alt="" className={mediaClass} />
    );
  };

  const areaVariantClass =
    variant === "avatar"
      ? `fs-mm-slot__area--avatar fs-mm-slot__area--avatar-${avatarSize}${isAvatarEdge ? " fs-mm-slot__area--edge" : ""}`
      : variant === "logo"
        ? "fs-mm-slot__area--logo"
        : "fs-mm-slot__area--banner";

  const actionButton = (
    <IconButton
      icon={hasMedia ? "Camera" : "Plus"}
      variant={isAvatarEdge ? "secondaryCircle" : "secondary"}
      size={isAvatarEdge ? AVATAR_EDGE_BUTTON_SIZE[avatarSize] : "md"}
      onClick={onActionClick}
      disabled={disabled}
      ariaLabel={hasMedia ? "Cambiar foto" : "Agregar foto"}
      data-test-id={`${testId}-action`}
    />
  );

  const dropZone = (
    <div
      className={`fs-mm-slot__area ${areaVariantClass}${isDragOver ? " fs-mm-slot__area--drag" : ""}`}
      onClick={openFile}
      onDragOver={(e) => {
        if (allowDragDrop) {
          e.preventDefault();
          setIsDragOver(true);
        }
      }}
      onDragLeave={() => setIsDragOver(false)}
      onDrop={onDrop}
      role="button"
      tabIndex={disabled ? -1 : 0}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          openFile();
        }
      }}
      aria-label={hasMedia ? "Actualizar imagen" : "Seleccionar imagen"}
      data-test-id={`${testId}-preview`}
    >
      {renderPreview()}
    </div>
  );

  return (
    <div
      className={`fs-mm-slot${isAvatarEdge ? " fs-mm-slot--edge" : ""}${disabled ? " fs-mm-slot--disabled" : ""} ${className}`.trim()}
      data-test-id={testId}
      data-action-placement={variant === "avatar" ? actionPlacement : undefined}
    >
      {isAvatarEdge ? (
        <div className="fs-mm-slot__edge-wrap">
          {dropZone}
          <div
            className={`fs-mm-slot__edge-action fs-mm-slot__edge-action--${avatarSize}`}
          >
            {actionButton}
          </div>
        </div>
      ) : (
        <>
          {dropZone}
          {actionButton}
        </>
      )}
      <input
        id={inputId}
        ref={fileInputRef}
        type="file"
        accept={acceptedTypes.join(",")}
        className="sr-only"
        disabled={disabled}
        onChange={onInputChange}
      />
      {error ? <Alert variant="error">{error}</Alert> : null}
    </div>
  );
}

export default MultimediaSingleSlot;
