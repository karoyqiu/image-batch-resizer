import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { message, open } from '@tauri-apps/plugin-dialog';
import { FolderOpen, Images, Plus, Trash2 } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Progress } from '@/components/ui/progress';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';

import '@/App.css';

type Format = 'png' | 'jpg';

interface Rule {
  id: number;
  width: string;
  height: string;
  format: Format;
  suffix: string;
}

interface ProgressPayload {
  done: number;
  total: number;
}

interface Summary {
  succeeded: number;
  failed: number;
  skipped: number;
}

const IMAGE_FILTERS = [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg'] }];

function basename(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/** Apply the system light/dark color scheme by toggling the `.dark` class. */
function useSystemTheme() {
  useEffect(() => {
    const root = document.documentElement;
    const query = window.matchMedia('(prefers-color-scheme: dark)');
    const apply = () => root.classList.toggle('dark', query.matches);
    apply();
    query.addEventListener('change', apply);
    return () => query.removeEventListener('change', apply);
  }, []);
}

function App() {
  useSystemTheme();

  const [sources, setSources] = useState<string[]>([]);
  const [dest, setDest] = useState('');
  const [rules, setRules] = useState<Rule[]>([]);
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<ProgressPayload | null>(null);
  const nextId = useRef(1);

  useEffect(() => {
    const unlisten: Array<() => void> = [];
    listen<ProgressPayload>('progress', (e) => setProgress(e.payload)).then((fn) =>
      unlisten.push(fn),
    );
    listen<Summary>('finished', (e) => {
      setRunning(false);
      setProgress(null);
      const { succeeded, failed, skipped } = e.payload;
      const stopped = skipped > 0;
      const body = stopped
        ? `Stopped early. ${succeeded} output(s) written, ${failed} failed, ${skipped} skipped.`
        : `Done. ${succeeded} output(s) written, ${failed} failed.`;
      void message(body, {
        title: stopped ? 'Stopped' : 'Finished',
        kind: failed > 0 ? 'warning' : 'info',
      });
    }).then((fn) => unlisten.push(fn));
    return () => unlisten.forEach((fn) => fn());
  }, []);

  const rulesValid =
    rules.length > 0 && rules.every((r) => Number(r.width) > 0 && Number(r.height) > 0);
  const canStart = sources.length > 0 && dest !== '' && rulesValid;

  async function selectSources() {
    const selected = await open({ multiple: true, filters: IMAGE_FILTERS });
    if (selected) setSources(Array.isArray(selected) ? selected : [selected]);
  }

  async function browseDest() {
    const selected = await open({ directory: true });
    if (selected) setDest(selected);
  }

  function addRule() {
    setRules((prev) => [
      ...prev,
      { id: nextId.current++, width: '', height: '', format: 'png', suffix: '' },
    ]);
  }

  function updateRule(id: number, patch: Partial<Rule>) {
    setRules((prev) => prev.map((r) => (r.id === id ? { ...r, ...patch } : r)));
  }

  function removeRule(id: number) {
    setRules((prev) => prev.filter((r) => r.id !== id));
  }

  async function onSubmit() {
    if (running) {
      void invoke('stop_batch');
      return;
    }
    setRunning(true);
    setProgress({ done: 0, total: sources.length * rules.length });
    void invoke('start_batch', {
      sources,
      dest,
      rules: rules.map((r) => ({
        width: Number(r.width),
        height: Number(r.height),
        format: r.format,
        suffix: r.suffix,
      })),
    }).catch(async (err: unknown) => {
      setRunning(false);
      setProgress(null);
      await message(String(err), { title: 'Cannot start', kind: 'error' });
    });
  }

  return (
    <main className="mx-auto flex h-dvh max-w-3xl flex-col gap-6 p-6">
      <h1 className="text-xl font-semibold">Image Batch Resizer</h1>

      <fieldset className="flex flex-col gap-2" disabled={running}>
        <Label>Source files</Label>
        <div className="flex items-center gap-2">
          <Button type="button" variant="outline" onClick={selectSources}>
            <Images /> Select files
          </Button>
          <span className="text-sm text-muted-foreground">
            {sources.length > 0 ? `${sources.length} file(s) selected` : 'None'}
          </span>
        </div>
        {sources.length > 0 && (
          <ul className="max-h-24 overflow-auto text-sm text-muted-foreground">
            {sources.map((p) => (
              <li key={p} className="truncate">
                {basename(p)}
              </li>
            ))}
          </ul>
        )}
      </fieldset>

      <fieldset className="flex flex-col gap-2" disabled={running}>
        <Label htmlFor="dest">Destination directory</Label>
        <div className="flex items-center gap-2">
          <Input
            id="dest"
            value={dest}
            onChange={(e) => setDest(e.target.value)}
            placeholder="Choose a folder…"
          />
          <Button type="button" variant="outline" onClick={browseDest}>
            <FolderOpen /> Browse
          </Button>
        </div>
      </fieldset>

      <fieldset className="flex flex-col gap-2" disabled={running}>
        <div className="flex items-center justify-between">
          <Label>Resize rules</Label>
          <Button type="button" variant="outline" size="sm" onClick={addRule}>
            <Plus /> Add rule
          </Button>
        </div>
        {rules.length > 0 && (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Width</TableHead>
                <TableHead>Height</TableHead>
                <TableHead>Format</TableHead>
                <TableHead>Suffix</TableHead>
                <TableHead className="w-10" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {rules.map((r) => (
                <TableRow key={r.id}>
                  <TableCell>
                    <Input
                      type="number"
                      min={1}
                      className="w-24"
                      value={r.width}
                      onChange={(e) => updateRule(r.id, { width: e.target.value })}
                    />
                  </TableCell>
                  <TableCell>
                    <Input
                      type="number"
                      min={1}
                      className="w-24"
                      value={r.height}
                      onChange={(e) => updateRule(r.id, { height: e.target.value })}
                    />
                  </TableCell>
                  <TableCell>
                    <Select
                      value={r.format}
                      onValueChange={(v) => updateRule(r.id, { format: v as Format })}
                    >
                      <SelectTrigger className="w-24">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="png">PNG</SelectItem>
                        <SelectItem value="jpg">JPG</SelectItem>
                      </SelectContent>
                    </Select>
                  </TableCell>
                  <TableCell>
                    <Input
                      className="w-32"
                      value={r.suffix}
                      onChange={(e) => updateRule(r.id, { suffix: e.target.value })}
                    />
                  </TableCell>
                  <TableCell>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => removeRule(r.id)}
                    >
                      <Trash2 />
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </fieldset>

      <div className="mt-auto flex flex-col gap-3">
        {running && progress && (
          <div className="flex items-center gap-3">
            <Progress value={progress.total ? (progress.done / progress.total) * 100 : 0} />
            <span className="w-24 shrink-0 text-right text-sm text-muted-foreground">
              {progress.done}/{progress.total}
            </span>
          </div>
        )}
        <Button type="button" size="lg" disabled={!running && !canStart} onClick={onSubmit}>
          {running ? 'Stop' : 'Start'}
        </Button>
      </div>
    </main>
  );
}

export default App;
