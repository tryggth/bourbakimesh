/**
 * Web Crypto Ephemeral Node Identity & Signed Proof Attestation Service.
 *
 * Implements Issue #30: Ephemeral cryptographic keypair generation in browser memory,
 * deterministic peer ID derivation, and digital signatures for extracted proof terms.
 */

export interface BrowserIdentity {
  peerId: string;
  publicKeyHex: string;
  algorithm: string;
  createdAt: number;
}

export interface SignedProofAttestation {
  proofHash: string;
  signatureHex: string;
  publicKeyHex: string;
  proverPeerId: string;
  timestamp: number;
}

let cachedKeyPair: CryptoKeyPair | null = null;
let cachedIdentity: BrowserIdentity | null = null;

/**
 * Convert ArrayBuffer to Hex String.
 */
export function bufToHex(buffer: ArrayBuffer | Uint8Array): string {
  const bytes = buffer instanceof Uint8Array ? buffer : new Uint8Array(buffer);
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

/**
 * Convert Hex String to Uint8Array.
 */
export function hexToBuf(hex: string): Uint8Array {
  const clean = hex.startsWith('0x') ? hex.slice(2) : hex;
  const bytes = new Uint8Array(clean.length / 2);
  for (let i = 0; i < clean.length; i += 2) {
    bytes[i / 2] = parseInt(clean.substring(i, i + 2), 16);
  }
  return bytes;
}

/**
 * Compute SHA-256 hash of a string or buffer.
 */
export async function sha256Hex(data: string | Uint8Array): Promise<string> {
  const bytes = typeof data === 'string' ? new TextEncoder().encode(data) : data;
  const subtle = getSubtleCrypto();
  const digest = await subtle.digest('SHA-256', bytes as unknown as BufferSource);
  return bufToHex(digest);
}

/**
 * Get SubtleCrypto instance with universal environment fallback.
 */
function getSubtleCrypto(): SubtleCrypto {
  if (typeof crypto !== 'undefined' && crypto.subtle) {
    return crypto.subtle;
  }
  if (typeof window !== 'undefined' && window.crypto && window.crypto.subtle) {
    return window.crypto.subtle;
  }
  throw new Error('Web Crypto API (crypto.subtle) is not available in this environment');
}

/**
 * Initialize or retrieve the ephemeral in-memory ECDSA P-256 keypair for this browser session.
 */
export async function getOrCreateBrowserIdentity(): Promise<BrowserIdentity> {
  if (cachedIdentity && cachedKeyPair) {
    return cachedIdentity;
  }

  const subtle = getSubtleCrypto();

  const keyPair = await subtle.generateKey(
    {
      name: 'ECDSA',
      namedCurve: 'P-256',
    },
    true, // extractable
    ['sign', 'verify']
  );

  const rawPubKey = await subtle.exportKey('raw', keyPair.publicKey);
  const pubKeyHex = bufToHex(rawPubKey);

  // Derive deterministic peer_id from public key SHA-256 digest
  const hashDigest = await subtle.digest('SHA-256', rawPubKey);
  const hashHex = bufToHex(hashDigest);
  const peerId = `browser-12D3KooW${hashHex.substring(0, 16)}`;

  cachedKeyPair = keyPair;
  cachedIdentity = {
    peerId,
    publicKeyHex: pubKeyHex,
    algorithm: 'ECDSA-P256-SHA256',
    createdAt: Date.now(),
  };

  return cachedIdentity;
}

/**
 * Sign an extracted proof term or block payload using the in-memory private key.
 */
export async function signProof(payload: string | Record<string, unknown>): Promise<SignedProofAttestation> {
  const identity = await getOrCreateBrowserIdentity();
  if (!cachedKeyPair) {
    throw new Error('Crypto keypair not initialized');
  }

  const payloadStr = typeof payload === 'string' ? payload : JSON.stringify(payload);
  const proofHash = await sha256Hex(payloadStr);

  const subtle = getSubtleCrypto();
  const payloadBytes = new TextEncoder().encode(payloadStr);

  const signatureBuf = await subtle.sign(
    {
      name: 'ECDSA',
      hash: { name: 'SHA-256' },
    },
    cachedKeyPair.privateKey,
    payloadBytes
  );

  return {
    proofHash,
    signatureHex: bufToHex(signatureBuf),
    publicKeyHex: identity.publicKeyHex,
    proverPeerId: identity.peerId,
    timestamp: Date.now() / 1000,
  };
}

/**
 * Verify a signed proof attestation using the sender's public key.
 */
export async function verifyProofSignature(
  payload: string | Record<string, unknown>,
  signatureHex: string,
  publicKeyHex: string
): Promise<boolean> {
  try {
    const subtle = getSubtleCrypto();
    const pubKeyBytes = hexToBuf(publicKeyHex);
    const signatureBytes = hexToBuf(signatureHex);

    const publicKey = await subtle.importKey(
      'raw',
      pubKeyBytes as unknown as BufferSource,
      {
        name: 'ECDSA',
        namedCurve: 'P-256',
      },
      true,
      ['verify']
    );

    const payloadStr = typeof payload === 'string' ? payload : JSON.stringify(payload);
    const payloadBytes = new TextEncoder().encode(payloadStr);

    return await subtle.verify(
      {
        name: 'ECDSA',
        hash: { name: 'SHA-256' },
      },
      publicKey,
      signatureBytes as unknown as BufferSource,
      payloadBytes as unknown as BufferSource
    );
  } catch (err) {
    console.warn('[Crypto] Signature verification failed:', err);
    return false;
  }
}
