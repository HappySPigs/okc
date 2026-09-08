import { lstat, mkdir, realpath } from 'node:fs/promises';
import path from 'node:path';
import { defaultConfig } from './config.js';
import { VaultError } from './vault.js';

/** Explicit bootstrap only. Never runs during config, doctor, or serve. */
export async function initializeSourceVault(vaultPath: string) {
  if (!path.isAbsolute(vaultPath)) throw new VaultError('INVALID_PATH', 'Use an absolute new Vault path.');
  const requested = path.resolve(vaultPath);
  const name = path.basename(requested);
  if (name.startsWith('.') || name.toLowerCase().endsWith('.okc-project') || /[<>:"|?*\x00-\x1f\x7f]/u.test(name)) {
    throw new VaultError('INVALID_PATH', 'Use a visible source Vault directory name.');
  }
  const parent = path.dirname(requested);
  const parentStat = await lstat(parent);
  if (parentStat.isSymbolicLink() || !parentStat.isDirectory()) {
    throw new VaultError('INVALID_PATH', 'The parent must be an existing real directory.');
  }
  const canonicalParent = await realpath(parent);
  let ancestor = canonicalParent;
  for (;;) {
    if (ancestor.toLowerCase().endsWith('.okc-project')) {
      throw new VaultError('IMMUTABLE_TARGET', 'Cannot initialize a source inside an OKC project.');
    }
    try {
      await lstat(path.join(ancestor, '.okc'));
      throw new VaultError('IMMUTABLE_TARGET', 'Cannot initialize a source inside a compiled artifact.');
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
    }
    const next = path.dirname(ancestor);
    if (next === ancestor) break;
    ancestor = next;
  }
  const target = path.join(canonicalParent, name);
  try { await mkdir(target, { mode: 0o700 }); }
  catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'EEXIST') throw new VaultError('NOTE_EXISTS', 'Vault target already exists; initialization never modifies an existing Vault.');
    throw error;
  }
  // Empty conventional folders only: no fabricated knowledge or operational docs.
  const folders = ['inbox', 'notes', 'sources', 'maps'];
  for (const folder of folders) await mkdir(path.join(target, folder), { mode: 0o700 });
  return { vaultPath: target, folders, config: defaultConfig(target),
    template: 'okc://templates/source-note', note: 'Save configuration outside the Vault. Folders are optional conventions, not policy or approval.' };
}
