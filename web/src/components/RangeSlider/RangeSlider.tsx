"use client";

import React, { useState, useRef, useEffect } from "react";
import "./range-slider.css";

interface RangeSliderProps {
    min?: number;
    max?: number;
    value?: [number, number];
    onChange?: (values: [number, number]) => void;
}

const RangeSlider: React.FC<RangeSliderProps> = ({ min = 0, max = 100, value, onChange }) => {
    const [values, setValues] = useState<[number, number]>(value || [min, max]);
    const [dragging, setDragging] = useState<null | "min" | "max">(null);
    const trackRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        if (value) setValues(value);
    }, [value]);

    useEffect(() => {
        const handleMouseUp = () => setDragging(null);
        window.addEventListener("mouseup", handleMouseUp);
        return () => window.removeEventListener("mouseup", handleMouseUp);
    }, []);

    useEffect(() => {
        const handleMouseMove = (e: MouseEvent) => {
            if (!dragging || !trackRef.current) return;
            const track = trackRef.current;
            const rect = track.getBoundingClientRect();
            const percent = Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width));
            const newValue = Math.round(percent * (max - min) + min);
            if (dragging === "min") {
                const clamped = Math.min(newValue, values[1]);
                setValues([clamped, values[1]]);
                onChange?.([clamped, values[1]]);
            } else {
                const clamped = Math.max(newValue, values[0]);
                setValues([values[0], clamped]);
                onChange?.([values[0], clamped]);
            }
        };
        if (dragging) {
            window.addEventListener("mousemove", handleMouseMove);
            return () => window.removeEventListener("mousemove", handleMouseMove);
        }
    }, [dragging, min, max, values, onChange]);

    const getPercent = (val: number) => ((val - min) / (max - min)) * 100;

    return (
        <div className="w-full" data-test-id="range-slider-root">
            <div className="mb-1 flex items-center justify-between">
                <span className="text-xs text-foreground font-light" data-test-id="range-slider-label">Rango</span>
            </div>
            <div className="fs-range-slider" ref={trackRef} data-test-id="range-slider-track">
                {/* Track */}
                <div className="fs-range-slider__track" data-test-id="range-slider-bg" />
                {/* Filled Range */}
                <div
                    className="fs-range-slider__fill"
                    style={{ left: `${getPercent(values[0])}%`, width: `${getPercent(values[1]) - getPercent(values[0])}%` }}
                    data-test-id="range-slider-fill"
                />
                {/* Thumbs */}
                <button
                    type="button"
                    className="fs-range-slider__thumb"
                    style={{ left: `${getPercent(values[0])}%`, transform: "translate(-50%, -50%)" }}
                    onMouseDown={() => setDragging("min")}
                    aria-label="Min value"
                    data-test-id="range-slider-thumb-min"
                />
                <button
                    type="button"
                    className="fs-range-slider__thumb"
                    style={{ left: `${getPercent(values[1])}%`, transform: "translate(-50%, -50%)" }}
                    onMouseDown={() => setDragging("max")}
                    aria-label="Max value"
                    data-test-id="range-slider-thumb-max"
                />
            </div>
            <div className="flex w-full justify-between mt-2" data-test-id="range-slider-values">
                <span className="text-xs text-foreground font-light" data-test-id="range-slider-value-min">{values[0]}</span>
                <span className="text-xs text-foreground font-light" data-test-id="range-slider-value-max">{values[1]}</span>
            </div>
        </div>
    );
};

export default RangeSlider;
