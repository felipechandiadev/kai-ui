"use client";
import React from 'react';
import * as Icons from 'lucide-react';
import './icon-button.css';

type IconButtonSize = 'xs' | 'sm' | 'md' | 'lg' | 'xl' | number;
type LucideIconName = keyof typeof Icons;

export type IconButtonVariant =
	| 'action'
	| 'primary'
	| 'secondary'
	| 'primaryCircle'
	| 'secondaryCircle'
	| 'neutral'
	| 'text'
	| 'outlined';

/** @deprecated Use IconButtonVariant names (action, primary, secondary, neutral). */
type IconButtonVariantLegacy =
	| 'containedPrimary'
	| 'containedSecondary'
	| 'basic'
	| 'basicSecondary'
	| 'ghost';

interface IconButtonProps {
	icon: LucideIconName;
	variant?: IconButtonVariant | IconButtonVariantLegacy;
	size?: IconButtonSize;
	/** Grosor del trazo del icono Lucide (por defecto 2.25, para igualar la intensidad de los iconos sueltos de la app). Valores mayores = icono más “fuerte”. */
	strokeWidth?: number;
	disabled?: boolean;
	isLoading?: boolean;
	className?: string;
	/** Clases aplicadas al SVG del icono (p. ej. `fill-current` en estrella destacada). */
	iconClassName?: string;
	onClick?: React.MouseEventHandler<HTMLButtonElement>;
	ariaLabel?: string;
	[key: string]: any;
}

const variantClassNames: Record<IconButtonVariant, string> = {
	action: 'fs-icon-button fs-icon-button--action',
	primary: 'fs-icon-button fs-icon-button--primary',
	secondary: 'fs-icon-button fs-icon-button--secondary',
	primaryCircle: 'fs-icon-button fs-icon-button--primary-circle',
	secondaryCircle: 'fs-icon-button fs-icon-button--secondary-circle',
	neutral: 'fs-icon-button fs-icon-button--neutral',
	text: 'fs-icon-button fs-icon-button--text',
	outlined: 'fs-icon-button fs-icon-button--outlined',
};

const legacyVariantMap: Record<IconButtonVariantLegacy, IconButtonVariant> = {
	containedPrimary: 'primary',
	containedSecondary: 'secondary',
	basic: 'neutral',
	basicSecondary: 'action',
	ghost: 'neutral',
};

function resolveVariantClass(variant: IconButtonProps['variant']): string {
	const key = variant ?? 'action';
	if (key in legacyVariantMap) {
		return variantClassNames[legacyVariantMap[key as IconButtonVariantLegacy]];
	}
	return variantClassNames[key as IconButtonVariant] ?? variantClassNames.action;
}

const sizeMap: Record<Exclude<IconButtonSize, number>, string> = {
	xs: 'w-5 h-5',
	sm: 'w-7 h-7',
	md: 'w-10 h-10',
	lg: 'w-12 h-12',
	xl: 'w-14 h-14',
};

const iconSizeMap: Record<Exclude<IconButtonSize, number>, number> = {
	xs: 16,
	sm: 18,
	md: 24,
	lg: 28,
	xl: 32,
};

const IconButton: React.FC<IconButtonProps> = ({
	icon,
	variant = 'action',
	size = 'md',
	strokeWidth = 2.25,
	disabled = false,
	isLoading = false,
	className = '',
	iconClassName = '',
	onClick,
	ariaLabel,
	...props
}) => {
	const sizeClass = typeof size === 'number' ? '' : sizeMap[size] || sizeMap['md'];
	const iconSize = typeof size === 'number' ? size : iconSizeMap[size] || 24;
	const effectiveDisabled = disabled || isLoading;
	const variantClass = resolveVariantClass(variant);

	const IconComponent = (
		isLoading ? Icons.Loader2 : Icons[icon]
	) as React.ComponentType<any>;

	if (!IconComponent) {
		console.warn(`Icon "${icon}" not found in lucide-react`);
		return (
			<button
				type="button"
				className={`${variantClass} inline-flex items-center justify-center ${sizeClass} ${
					effectiveDisabled ? 'opacity-50' : ''
				} ${className}`}
				data-test-id="icon-button-root"
				onClick={onClick}
				aria-label={ariaLabel}
				disabled={effectiveDisabled}
				{...props}
			>
				<span className="text-lg">?</span>
			</button>
		);
	}

	return (
		<button
			type="button"
			className={`${variantClass} inline-flex items-center justify-center ${sizeClass} ${
				effectiveDisabled ? 'opacity-50' : ''
			} ${className}`}
			data-test-id="icon-button-root"
			onClick={onClick}
			aria-label={ariaLabel}
			aria-busy={isLoading || undefined}
			disabled={effectiveDisabled}
			{...props}
		>
			<IconComponent
				size={iconSize}
				strokeWidth={strokeWidth}
				className={`select-none ${isLoading ? 'fs-icon-button__icon--loading' : ''} ${iconClassName}`.trim()}
				aria-hidden
			/>
		</button>
	);
};

export default IconButton;
