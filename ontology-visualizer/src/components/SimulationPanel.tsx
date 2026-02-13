
import React, { useState, useEffect } from 'react';
import type { SimulationStep } from '../services/simulationService';
import { PlayCircle, GitCommit, FileText, CheckCircle, ArrowRight, Pause, SkipBack } from 'lucide-react';

interface SimulationPanelProps {
    isOpen: boolean;
    onClose: () => void;
    steps: SimulationStep[];
    currentStepIndex: number;
    runSimulation: () => void;
    onStepClick?: (step: SimulationStep) => void;
    onSetStepIndex: React.Dispatch<React.SetStateAction<number>>;
}

const SimulationPanel: React.FC<SimulationPanelProps> = ({
    isOpen,
    onClose,
    steps,
    currentStepIndex,
    runSimulation,
    onStepClick,
    onSetStepIndex
}) => {
    const [isPlaying, setIsPlaying] = useState(false);

    useEffect(() => {
        let interval: any;
        if (isPlaying && steps.length > 0) {
            interval = setInterval(() => {
                onSetStepIndex((prev: number) => {
                    if (prev >= steps.length - 1) {
                        setIsPlaying(false);
                        return prev;
                    }
                    return prev + 1;
                });
            }, 1000);
        }
        return () => clearInterval(interval);
    }, [isPlaying, steps, onSetStepIndex]);

    if (!isOpen) return null;

    const handleBack = () => {
        setIsPlaying(false);
        onSetStepIndex(Math.max(-1, currentStepIndex - 1));
    };

    const handleForward = () => {
        setIsPlaying(false);
        onSetStepIndex(Math.min(steps.length - 1, currentStepIndex + 1));
    };

    const handleReset = () => {
        setIsPlaying(false);
        onSetStepIndex(-1);
    };

    return (
        <aside className="side-panel floating left-panel" style={{
            position: 'absolute',
            top: '80px',
            left: '20px',
            bottom: '20px',
            width: '320px',
            background: 'rgba(22, 27, 34, 0.95)',
            backdropFilter: 'blur(10px)',
            border: '1px solid rgba(255, 255, 255, 0.1)',
            borderRadius: '8px',
            display: 'flex',
            flexDirection: 'column',
            zIndex: 1000,
            color: '#c9d1d9'
        }}>
            <div className="panel-header" style={{
                padding: '12px 16px',
                borderBottom: '1px solid rgba(255, 255, 255, 0.1)',
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center'
            }}>
                <h2 style={{ fontSize: '1rem', margin: 0, display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <PlayCircle size={18} color="#58a6ff" />
                    Execution Simulation
                </h2>
                <button className="btn" style={{ padding: '4px', background: 'transparent', border: 'none', color: '#c9d1d9', cursor: 'pointer' }} onClick={onClose}>
                    &times;
                </button>
            </div>

            <div style={{ padding: '12px', borderBottom: '1px solid rgba(255, 255, 255, 0.1)', display: 'flex', flexDirection: 'column', gap: '10px' }}>
                <button
                    onClick={runSimulation}
                    className="btn btn-primary"
                    style={{
                        width: '100%',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        gap: '8px',
                        padding: '8px',
                        background: '#238636',
                        color: 'white',
                        border: '1px solid rgba(255, 255, 255, 0.1)',
                        borderRadius: '6px',
                        cursor: 'pointer',
                        fontWeight: 600
                    }}
                >
                    <PlayCircle size={16} /> {steps.length > 0 ? 'Restart Simulation' : 'Run Simulation'}
                </button>

                {steps.length > 0 && (
                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '8px' }}>
                        <button className="btn btn-sm" onClick={handleReset} title="Reset">
                            <SkipBack size={14} />
                        </button>
                        <button className="btn btn-sm" onClick={handleBack} disabled={currentStepIndex <= -1}>
                            <ArrowRight size={14} style={{ transform: 'rotate(180deg)' }} />
                        </button>
                        <button
                            className="btn btn-sm btn-primary"
                            onClick={() => setIsPlaying(!isPlaying)}
                            style={{ padding: '8px 16px' }}
                        >
                            {isPlaying ? <Pause size={16} /> : <PlayCircle size={16} />}
                        </button>
                        <button className="btn btn-sm" onClick={handleForward} disabled={currentStepIndex >= steps.length - 1}>
                            <ArrowRight size={14} />
                        </button>
                    </div>
                )}
            </div>

            <div className="panel-content" style={{ flex: 1, overflowY: 'auto', padding: '12px' }}>
                {steps.length === 0 ? (
                    <div style={{ textAlign: 'center', padding: '20px', color: '#8b949e', fontSize: '0.9rem' }}>
                        Click "Run Simulation" to see the predicted execution path.
                    </div>
                ) : (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                        {steps.map((step, index) => (
                            <div
                                key={step.id}
                                onClick={() => {
                                    setIsPlaying(false);
                                    onStepClick && onStepClick(step);
                                }}
                                style={{
                                    background: index === currentStepIndex ? 'rgba(56, 139, 253, 0.15)' : 'rgba(13, 17, 23, 0.5)',
                                    border: index === currentStepIndex ? '1px solid #58a6ff' : '1px solid rgba(48, 54, 61, 0.7)',
                                    borderRadius: '6px',
                                    padding: '10px',
                                    cursor: 'pointer',
                                    transition: 'all 0.2s',
                                    fontSize: '0.85rem',
                                    opacity: index > currentStepIndex ? 0.5 : 1
                                }}
                            >
                                <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginBottom: '6px' }}>
                                    <span style={{
                                        background: '#30363d',
                                        borderRadius: '4px',
                                        padding: '2px 6px',
                                        fontSize: '0.7rem',
                                        color: '#8b949e'
                                    }}>
                                        #{step.id}
                                    </span>
                                    {step.verbType === 'Creation' && <GitCommit size={14} color="#7ee787" />}
                                    {step.verbType === 'Verification' && <CheckCircle size={14} color="#f0883e" />}
                                    {step.verbType === 'Refinement' && <ArrowRight size={14} color="#d2a8ff" />}

                                    <span style={{ fontWeight: 600, color: '#58a6ff' }}>{step.agent}</span>
                                    <span style={{ color: '#8b949e', fontStyle: 'italic' }}>{step.verb}</span>
                                </div>

                                <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                                    <FileText size={14} color="#79c0ff" />
                                    <span style={{ fontWeight: 600, color: '#c9d1d9' }}>{step.target}</span>
                                </div>
                            </div>
                        ))}
                    </div>
                )}
            </div>
        </aside>
    );
};

export default SimulationPanel;
