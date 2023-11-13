import os from 'os';
import path from 'path';
import fs from 'fs';
import {generateKeyPair} from "../ssl/generateKeyPair.js";
import {generateCsr} from "../ssl/zerossl/generateCsr.js";
import {createSelfSignedCertificate as createCertificate} from "../ssl/selfSigned/createSelfSignedCertificate";

// TODO: Refactor to reuse the logic together with obtainSelfSignedCertificateTask
export async function createSelfSignedCertificate(ip) {
  const keyPair = await generateKeyPair();
  const csr = await generateCsr(keyPair, ip);
  const certificate = await createCertificate(keyPair, csr);

  const tempDir = os.tmpdir();
  const certificatePath = path.join(tempDir, 'bundle.crt');
  const privKeyPath = path.join(tempDir, 'private.key');
  fs.writeFileSync(certificatePath, certificate, 'utf8');
  fs.writeFileSync(privKeyPath, keyPair.privateKey, 'utf8');
  return { certificatePath, privKeyPath };
}
