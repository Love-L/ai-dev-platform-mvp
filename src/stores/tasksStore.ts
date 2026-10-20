import { create } from 'zustand';
import { listTasks, createTask, updateTaskStatus, deleteTask } from '../lib/tauri';
import type { Task } from '../lib/types';

interface TasksState {
  tasks: Task[];
  loading: boolean;
  error: string | null;
  fetchTasks: (status?: string, taskType?: string) => Promise<void>;
  addTask: (title: string, requirementId?: string, taskType?: string, priority?: string) => Promise<Task>;
  updateStatus: (id: string, status: string) => Promise<void>;
  removeTask: (id: string) => Promise<void>;
}

export const useTasksStore = create<TasksState>((set, get) => ({
  tasks: [],
  loading: false,
  error: null,

  fetchTasks: async (status, taskType) => {
    set({ loading: true, error: null });
    try {
      const data = await listTasks(status, taskType);
      set({ tasks: data, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  addTask: async (title, requirementId, taskType, priority) => {
    const task = await createTask(title, requirementId, taskType, priority);
    await get().fetchTasks();
    return task;
  },

  updateStatus: async (id, status) => {
    await updateTaskStatus(id, status);
    await get().fetchTasks();
  },

  removeTask: async (id) => {
    await deleteTask(id);
    await get().fetchTasks();
  },
}));
