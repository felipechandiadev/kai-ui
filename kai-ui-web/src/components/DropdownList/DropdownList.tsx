"use client";
import React, { useState, useEffect, useRef } from "react";
import { createPortal } from "react-dom";

import "./dropdown.css";

// Option row style for dropdown
export const dropdownOptionClass = "fs-dropdown__option";

interface DropdownListProps {
  open: boolean;
  children?: React.ReactNode;
  className?: string;
  style?: React.CSSProperties;
  testId?: string;
  dropUp?: boolean;
  // New props for hover management
  highlightedIndex?: number;
  onHoverChange?: (index: number) => void;
  renderItems?: boolean; // If true, DropdownList renders items internally
  // Reference to the trigger element for portal positioning
  anchorRef?: React.RefObject<HTMLElement | null>;
  // Use portal mode (renders at body level to escape overflow constraints)
  usePortal?: boolean;
}

const DropdownList: React.FC<DropdownListProps> = ({ 
  open, 
  children, 
  className = "", 
  style, 
  testId, 
  dropUp = false,
  highlightedIndex = -1,
  onHoverChange,
  anchorRef,
  usePortal = false,
}) => {
  const [hoveredIndex, setHoveredIndex] = useState<number>(-1);
  const [position, setPosition] = useState<{ top: number; left: number; width: number } | null>(null);
  const [mounted, setMounted] = useState(false);
  const dropdownRef = useRef<HTMLUListElement>(null);

  // Handle client-side mounting for portal
  useEffect(() => {
    setMounted(true);
  }, []);

  // Calculate position when using portal mode
  useEffect(() => {
    if (!usePortal || !open || !anchorRef?.current) {
      setPosition(null);
      return;
    }

    const updatePosition = () => {
      if (!anchorRef.current) return;
      
      const rect = anchorRef.current.getBoundingClientRect();
      const dropdownHeight = dropdownRef.current?.offsetHeight || 224; // max-h-56 = 224px
      const viewportHeight = window.innerHeight;
      
      // Check if dropdown would overflow bottom of viewport
      const spaceBelow = viewportHeight - rect.bottom;
      const spaceAbove = rect.top;
      const shouldDropUp = dropUp || (spaceBelow < dropdownHeight && spaceAbove > spaceBelow);
      
      setPosition({
        top: shouldDropUp ? rect.top - dropdownHeight - 4 : rect.bottom + 4,
        left: rect.left,
        width: rect.width,
      });
    };

    updatePosition();

    // Update position on scroll or resize
    window.addEventListener('scroll', updatePosition, true);
    window.addEventListener('resize', updatePosition);
    
    return () => {
      window.removeEventListener('scroll', updatePosition, true);
      window.removeEventListener('resize', updatePosition);
    };
  }, [open, usePortal, anchorRef, dropUp]);

  if (!open) return null;

  const handleMouseEnter = (index: number) => {
    setHoveredIndex(index);
    onHoverChange?.(index);
  };

  const handleMouseLeave = () => {
    setHoveredIndex(-1);
    onHoverChange?.(-1);
  };

  // If children are provided, render them with hover management
  const childrenArray = React.Children.toArray(children);

  const childrenWithHover = children
    ? childrenArray.map((child, idx) => {
        if (React.isValidElement<{ className?: string }>(child)) {
          const currentClassName = child.props.className || '';
          // UX: navegación por teclado debe verse igual que hover.
          const hoverClass =
            highlightedIndex === idx || hoveredIndex === idx ? "bg-neutral" : "";

          return React.cloneElement(child, {
            key: child.key ?? idx,
            className: `${currentClassName} ${hoverClass}`.trim(),
            onMouseEnter: () => handleMouseEnter(idx),
            onMouseLeave: () => handleMouseLeave(),
          } as any);
        }
        return child;
      })
    : children;

  // Base styles for non-portal mode
  const dropStyle = dropUp 
    ? { bottom: '100%', top: 'auto', marginBottom: '0.25rem', marginTop: 0 } 
    : { marginTop: '0.25rem' };

  // Portal styles (fixed positioning)
  const portalStyle: React.CSSProperties = position ? {
    position: 'fixed',
    top: position.top,
    left: position.left,
    width: position.width,
    zIndex: 9999, // Maximum z-index to ensure it's above everything
  } : {};

  const dropdownElement = (
    <ul
      ref={dropdownRef}
      className={`fs-dropdown__list ${usePortal ? "fs-dropdown__list--portal" : ""} ${className}`.trim()}
      style={usePortal && position ? { ...portalStyle, ...style } : (dropUp || Object.keys(style || {}).length > 0 ? { ...dropStyle, ...style } : { ...dropStyle })}
      data-test-id={testId || "dropdown-list"}
    >
      {childrenWithHover}
    </ul>
  );

  // Use portal for rendering outside the DOM hierarchy
  if (usePortal && mounted) {
    return createPortal(dropdownElement, document.body);
  }

  return dropdownElement;
};

export default DropdownList;
