import { useState, useEffect, useCallback } from "react";
import {
  TRAINING_PARAGRAPHS,
  TOTAL_PARAGRAPHS,
} from "@/lib/training-paragraphs";
import { startRecording, stopRecording } from "@/lib/commands/recording";
import { computeTrainingDiffs } from "@/lib/commands/corrections";
import { getSetting, setSetting } from "@/lib/commands/settings";
import { TrainingComplete } from "./TrainingComplete";

interface TrainingTabProps {
  onDone: () => void;
}

export function TrainingTab({ onDone }: TrainingTabProps) {
  const [paragraphIndex, setParagraphIndex] = useState(0);
  const [isRecording, setIsRecording] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  const [isComplete, setIsComplete] = useState(false);
  const [totalCorrectionsLearned, setTotalCorrectionsLearned] = useState(0);
  const [error, setError] = useState<string | null>(null);

  // Load saved progress on mount
  useEffect(() => {
    getSetting("training_paragraph_index")
      .then((val) => {
        const idx = parseInt(val, 10);
        if (!isNaN(idx) && idx >= 0 && idx < TOTAL_PARAGRAPHS) {
          setParagraphIndex(idx);
        }
      })
      .catch(() => {
        // Setting doesn't exist yet, start from 0
      });
  }, []);

  const persistIndex = useCallback((idx: number) => {
    void setSetting("training_paragraph_index", String(idx));
  }, []);

  const handleRecordToggle = async () => {
    setError(null);

    if (isRecording) {
      // Stop recording and process
      setIsRecording(false);
      setIsProcessing(true);
      try {
        const result = await stopRecording();
        const paragraph = TRAINING_PARAGRAPHS[paragraphIndex];
        if (paragraph && result.raw_text) {
          const count = await computeTrainingDiffs(paragraph, result.raw_text);
          setTotalCorrectionsLearned((prev) => prev + count);
        }

        // Advance to next paragraph
        const nextIndex = paragraphIndex + 1;
        if (nextIndex >= TOTAL_PARAGRAPHS) {
          setIsComplete(true);
          persistIndex(0);
        } else {
          setParagraphIndex(nextIndex);
          persistIndex(nextIndex);
        }
      } catch (e) {
        setError(
          e instanceof Error ? e.message : "Recording failed. Try again.",
        );
      }
      setIsProcessing(false);
    } else {
      // Start recording
      try {
        await startRecording();
        setIsRecording(true);
      } catch (e) {
        setError(
          e instanceof Error
            ? e.message
            : "Could not start recording. Check microphone access.",
        );
      }
    }
  };

  const handlePrev = () => {
    if (paragraphIndex > 0) {
      const newIdx = paragraphIndex - 1;
      setParagraphIndex(newIdx);
      persistIndex(newIdx);
    }
  };

  const handleNext = () => {
    if (paragraphIndex < TOTAL_PARAGRAPHS - 1) {
      const newIdx = paragraphIndex + 1;
      setParagraphIndex(newIdx);
      persistIndex(newIdx);
    }
  };

  const handleDone = () => {
    setIsComplete(false);
    setParagraphIndex(0);
    persistIndex(0);
    setTotalCorrectionsLearned(0);
    onDone();
  };

  const currentParagraph = TRAINING_PARAGRAPHS[paragraphIndex] ?? "";

  return (
    <section className="bg-white rounded-xl border border-black/[0.07] shadow-sm min-h-[460px] flex flex-col items-center justify-center text-center p-8">
      {isComplete ? (
        <TrainingComplete
          correctionsCount={totalCorrectionsLearned}
          onDone={handleDone}
        />
      ) : (
        <>
          <span className="text-[10px] font-semibold tracking-wider text-black/[0.26] uppercase mb-1">
            PROGRESS
          </span>
          <h2 className="text-[17px] font-semibold text-[#1C1C1E] mb-8">
            Paragraph {paragraphIndex + 1} of {TOTAL_PARAGRAPHS}
          </h2>

          {/* Paragraph text box */}
          <div className="max-w-[560px] w-full bg-white border border-black/[0.07] rounded-xl p-6 mb-6 shadow-sm">
            <p className="text-[13px] text-[#1C1C1E] leading-[1.7] text-left">
              {currentParagraph}
            </p>
          </div>

          <p className="text-[12px] text-black/[0.26] mb-6">
            Read the paragraph above aloud
          </p>

          {error && (
            <p className="text-[12px] text-[#ba1a1a] mb-4">{error}</p>
          )}

          <button
            onClick={() => void handleRecordToggle()}
            disabled={isProcessing}
            className={`text-[13px] font-medium w-[160px] h-[36px] rounded-lg shadow-sm transition-all mb-8 ${
              isRecording
                ? "bg-[#ba1a1a] text-white hover:brightness-110"
                : isProcessing
                  ? "bg-black/20 text-white cursor-not-allowed"
                  : "bg-[#0058bc] text-white hover:brightness-110"
            }`}
          >
            {isRecording
              ? "Stop Recording"
              : isProcessing
                ? "Processing..."
                : "Start Recording"}
          </button>

          {/* Previous / Next navigation */}
          <div className="flex items-center gap-4 text-[13px]">
            <button
              onClick={handlePrev}
              disabled={paragraphIndex <= 0 || isRecording || isProcessing}
              className={
                paragraphIndex <= 0 || isRecording || isProcessing
                  ? "text-black/[0.15] cursor-not-allowed font-medium"
                  : "text-[#0058bc] font-medium hover:underline"
              }
            >
              &larr; Previous
            </button>
            <span className="text-black/[0.26]">&middot;</span>
            <button
              onClick={handleNext}
              disabled={
                paragraphIndex >= TOTAL_PARAGRAPHS - 1 ||
                isRecording ||
                isProcessing
              }
              className={
                paragraphIndex >= TOTAL_PARAGRAPHS - 1 ||
                isRecording ||
                isProcessing
                  ? "text-black/[0.15] cursor-not-allowed font-medium"
                  : "text-[#0058bc] font-medium hover:underline"
              }
            >
              Next &rarr;
            </button>
          </div>
        </>
      )}
    </section>
  );
}
