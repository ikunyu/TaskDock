// Static artifact verification; does not start TaskDock or run its business logic.
import { readFile } from 'node:fs/promises';
import { createHash, createPublicKey, verify } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const read = (name) => readFile(path.join(root, name));
const config = JSON.parse(await read('src-tauri/tauri.conf.json'));
const release = JSON.parse(await read('release-config.json'));
const manifest = JSON.parse(await read('artifacts/release/latest.json'));
const pubText = Buffer.from(config.plugins.updater.pubkey, 'base64')
  .toString('utf8')
  .trim()
  .split(/\r?\n/);
const pub = Buffer.from(pubText[1], 'base64');
const lines = Buffer.from(manifest.platforms['windows-x86_64'].signature, 'base64')
  .toString('utf8')
  .trim()
  .split(/\r?\n/);
const packet = Buffer.from(lines[1], 'base64');
const globalSignature = Buffer.from(lines[3], 'base64');
if (
  pub.length !== 42 ||
  packet.length !== 74 ||
  globalSignature.length !== 64 ||
  !packet.subarray(2, 10).equals(pub.subarray(2, 10))
)
  throw Error('Signature key mismatch');
const algorithm = packet.subarray(0, 2).toString('ascii');
if (!['Ed', 'ED'].includes(algorithm) || !lines[2].startsWith('trusted comment: '))
  throw Error('Invalid minisign packet');
const comment = lines[2].slice('trusted comment: '.length);
if (
  manifest.appVersion !== release.appVersion ||
  config.version !== release.appVersion ||
  manifest.version !== `1.0.0+${release.build}` ||
  !comment.split('\t').includes(`version:${manifest.version}`)
)
  throw Error('Signed build/version mismatch');
const publicKey = createPublicKey({
  key: Buffer.concat([Buffer.from('302a300506032b6570032100', 'hex'), pub.subarray(10)]),
  format: 'der',
  type: 'spki',
});
const installer = await read('artifacts/release/TaskDock_1.0.0_x64-setup.exe');
const versionedInstaller = await read(
  `artifacts/release/TaskDock_${release.appVersion}_x64-setup.exe`,
);
if (!installer.equals(versionedInstaller)) throw Error('Compatibility installer alias differs');
const message =
  algorithm === 'ED' ? createHash('blake2b512').update(installer).digest() : installer;
if (
  !verify(null, message, publicKey, packet.subarray(10)) ||
  !verify(
    null,
    Buffer.concat([packet.subarray(10), Buffer.from(comment)]),
    publicKey,
    globalSignature,
  )
)
  throw Error('Update package signature verification failed');
console.log(`Verified installer signature and signed build ${manifest.version}`);
