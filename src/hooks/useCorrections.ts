// Corrections hook -- paginated CRUD with optimistic delete
import { useState, useEffect, useCallback } from "react";
import type { Correction } from "@/types/commands";
import {
  getCorrections,
  addCorrection,
  deleteCorrection,
  getCorrectionsCount,
} from "@/lib/commands/corrections";

const PAGE_SIZE = 20;

interface CorrectionsData {
  corrections: Correction[];
  totalCount: number;
  isLoading: boolean;
  currentPage: number;
  totalPages: number;
  nextPage: () => void;
  prevPage: () => void;
  addNewCorrection: (rawWord: string, corrected: string) => Promise<void>;
  removeCorrection: (id: string) => Promise<void>;
  refresh: () => void;
}

export function useCorrections(): CorrectionsData {
  const [corrections, setCorrections] = useState<Correction[]>([]);
  const [totalCount, setTotalCount] = useState(0);
  const [isLoading, setIsLoading] = useState(true);
  const [currentPage, setCurrentPage] = useState(1);

  const totalPages = Math.max(1, Math.ceil(totalCount / PAGE_SIZE));

  const fetchPage = useCallback(async (page: number) => {
    setIsLoading(true);
    try {
      const offset = (page - 1) * PAGE_SIZE;
      const [items, count] = await Promise.all([
        getCorrections(PAGE_SIZE, offset),
        getCorrectionsCount(),
      ]);
      setCorrections(items);
      setTotalCount(count);
      setCurrentPage(page);
    } catch {
      setCorrections([]);
      setTotalCount(0);
    }
    setIsLoading(false);
  }, []);

  useEffect(() => {
    void fetchPage(1);
  }, [fetchPage]);

  const nextPage = useCallback(() => {
    if (currentPage < totalPages) {
      void fetchPage(currentPage + 1);
    }
  }, [currentPage, totalPages, fetchPage]);

  const prevPage = useCallback(() => {
    if (currentPage > 1) {
      void fetchPage(currentPage - 1);
    }
  }, [currentPage, fetchPage]);

  const addNewCorrection = useCallback(
    async (rawWord: string, corrected: string) => {
      await addCorrection(rawWord, corrected);
      void fetchPage(currentPage);
    },
    [currentPage, fetchPage],
  );

  const removeCorrection = useCallback(
    async (id: string) => {
      setCorrections((prev) => prev.filter((c) => c.id !== id));
      try {
        await deleteCorrection(id);
        const count = await getCorrectionsCount();
        setTotalCount(count);
      } catch {
        void fetchPage(currentPage);
      }
    },
    [currentPage, fetchPage],
  );

  return {
    corrections,
    totalCount,
    isLoading,
    currentPage,
    totalPages,
    nextPage,
    prevPage,
    addNewCorrection,
    removeCorrection,
    refresh: () => void fetchPage(currentPage),
  };
}
