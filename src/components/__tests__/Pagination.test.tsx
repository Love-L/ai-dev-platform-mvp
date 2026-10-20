/**
 * Pagination组件单元测试
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import Pagination from "../Pagination";

describe("Pagination", () => {
  const mockOnPageChange = vi.fn();

  beforeEach(() => {
    mockOnPageChange.mockClear();
  });

  it("renders pagination with correct pages", () => {
    render(<Pagination currentPage={1} totalPages={5} onPageChange={mockOnPageChange} />);

    // 验证页码按钮存在
    expect(screen.getByText("1")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getByText("4")).toBeInTheDocument();
    expect(screen.getByText("5")).toBeInTheDocument();
  });

  it("calls onPageChange when clicking a page", () => {
    render(<Pagination currentPage={1} totalPages={5} onPageChange={mockOnPageChange} />);

    // 点击第2页
    fireEvent.click(screen.getByText("2"));
    expect(mockOnPageChange).toHaveBeenCalledWith(2);
  });

  it("disables previous button on first page", () => {
    render(<Pagination currentPage={1} totalPages={5} onPageChange={mockOnPageChange} />);

    // 找到ChevronLeft图标所在的按钮
    const buttons = screen.getAllByRole("button");
    const prevButton = buttons[0]; // 第一个按钮是上一页
    expect(prevButton).toBeDisabled();
  });

  it("disables next button on last page", () => {
    render(<Pagination currentPage={5} totalPages={5} onPageChange={mockOnPageChange} />);

    // 找到ChevronRight图标所在的按钮
    const buttons = screen.getAllByRole("button");
    const nextButton = buttons[buttons.length - 1]; // 最后一个按钮是下一页
    expect(nextButton).toBeDisabled();
  });

  it("calls onPageChange with previous page when clicking previous", () => {
    render(<Pagination currentPage={3} totalPages={5} onPageChange={mockOnPageChange} />);

    const buttons = screen.getAllByRole("button");
    const prevButton = buttons[0];
    fireEvent.click(prevButton);
    expect(mockOnPageChange).toHaveBeenCalledWith(2);
  });

  it("calls onPageChange with next page when clicking next", () => {
    render(<Pagination currentPage={3} totalPages={5} onPageChange={mockOnPageChange} />);

    const buttons = screen.getAllByRole("button");
    const nextButton = buttons[buttons.length - 1];
    fireEvent.click(nextButton);
    expect(mockOnPageChange).toHaveBeenCalledWith(4);
  });

  it("returns null when totalPages <= 1", () => {
    const { container } = render(<Pagination currentPage={1} totalPages={1} onPageChange={mockOnPageChange} />);

    expect(container.firstChild).toBeNull();
  });

  it("highlights current page", () => {
    render(<Pagination currentPage={2} totalPages={5} onPageChange={mockOnPageChange} />);

    const currentPageBtn = screen.getByText("2");
    expect(currentPageBtn).toHaveClass("bg-blue-600");
  });
});
