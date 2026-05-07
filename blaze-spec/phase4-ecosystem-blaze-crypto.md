# Phase 4 – Ecosystem Crate: `blaze‑crypto`

> **Goal:** Specify the `blaze‑crypto` crate, which provides a comprehensive set of cryptographic primitives: secure random number generation, hashing, message authentication, symmetric encryption, asymmetric encryption, digital signatures, and key derivation.  The crate is designed to be data‑oriented, linear, and zero‑cost where possible, with a focus on safety and misuse resistance.  It integrates with `blaze‑tls`, `blaze‑jwt`, and any other ecosystem crate requiring cryptographic operations.

---

## 1. Core Principles

- **Zero‑copy:** Inputs and outputs are provided as `&[u8]` and `Vec<u8>` without unnecessary copies.
- **Linear key types:** Secret keys are linear (cannot be copied implicitly) to prevent accidental leakage.  Public keys are `@copy`.
- **Deterministic:** All operations are pure (no I/O, no effect) except for random generation, which may use system entropy and thus carries the `io` effect (but is safe to call repeatedly).  For testing, a deterministic RNG is provided via `std::random`.
- **Resistant to timing attacks:** The implementation uses constant‑time operations where required (e.g., comparison of MACs, modular exponentiation for RSA).  For algorithms where timing resistance is inherently guaranteed by the design (e.g., AES‑GCM, ChaCha20‑Poly1305), the implementation follows the standard.

---

## 2. Secure Random

The crate re‑exports `std::random::thread_rng` and provides a cryptographically secure random number generator built on the system’s entropy source.

### 2.1 `SecureRandom`

```
pub struct SecureRandom { /* … */ }
impl SecureRandom {
    pub fn new() -> SecureRandom;
    pub fn fill_bytes(&mut self, dest: &mut [u8]);
}
```

- `new()` initializes from the system’s secure random source (e.g., `getrandom()` on Unix, `BCryptGenRandom` on Windows).  Carries `io` effect because it may block waiting for entropy.
- `fill_bytes` fills the provided buffer with cryptographically random bytes.  After initialisation, subsequent calls are pure and fast.

---

## 3. Hashing (SHA‑2 family, SHA‑3, BLAKE3)

### 3.1 `Hash` Trait

```
pub trait Hash {
    fn update(&mut self, data: &[u8]);
    fn finalize(self) -> Vec<u8>;
    fn output_size() -> usize;
    fn block_size() -> usize;
}
```

- `update` processes more input.
- `finalize(self)` consumes the hasher and returns the digest as a `Vec<u8>`.

### 3.2 Specific Hash Structs

```
pub struct Sha256 { … } impl Hash for Sha256 { … }
pub struct Sha512 { … } impl Hash for Sha512 { … }
pub struct Sha3_256 { … } impl Hash for Sha3_256 { … }
pub struct Sha3_512 { … } impl Hash for Sha3_512 { … }
pub struct Blake3 { … } impl Hash for Blake3 { … }
```

- Each struct can be created via `new()` or `default()`.
- `Blake3` additionally supports parallel hashing (using threads) if the `parallel` feature is enabled; otherwise it defaults to sequential.

### 3.3 Convenience Functions

```
pub fn sha256(data: &[u8]) -> Vec<u8>;
pub fn sha512(data: &[u8]) -> Vec<u8>;
// etc.
```

- One‑shot hashing without explicit hasher object.

---

## 4. Message Authentication (HMAC, Poly1305)

### 4.1 `Hmac`

```
pub struct Hmac<H: Hash> {
    inner: H,
    // key, ipad, opad precomputed
}
impl<H: Hash> Hmac<H> {
    pub fn new(key: &[u8]) -> Hmac<H>;
    pub fn update(&mut self, data: &[u8]);
    pub fn finalize(self) -> Vec<u8>;
    pub fn verify(self, tag: &[u8]) -> Result<(), MacError>;
}
```

- `verify` compares the computed tag with the provided tag in constant time.  Returns `MacError` if they differ.

### 4.2 `Poly1305`

```
pub struct Poly1305 { … }
impl Poly1305 {
    pub fn new(key: &[u8; 32]) -> Self;
    pub fn update(&mut self, data: &[u8]);
    pub fn finalize(self) -> [u8; 16];
    pub fn verify(self, tag: &[u8; 16]) -> Result<(), MacError>;
}
```

- One‑time authenticator, typically used with ChaCha20 (see AEAD).

---

## 5. Authenticated Encryption with Associated Data (AEAD)

### 5.1 `Aead` Trait

```
pub trait Aead {
    fn seal(&self, nonce: &[u8], plaintext: &[u8], aad: &[u8]) -> Vec<u8>;
    fn open(&self, nonce: &[u8], ciphertext: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptoError>;
}
```

- `seal` encrypts and authenticates, producing ciphertext with an appended authentication tag.
- `open` decrypts and verifies; returns the plaintext if authentication succeeds, otherwise `CryptoError`.

### 5.2 Specific AEAD Structs

```
pub struct Aes128Gcm { … }  // AES‑128‑GCM
pub struct Aes256Gcm { … }  // AES‑256‑GCM
pub struct ChaCha20Poly1305 { … }
```

- Constructed with a secret key (linear) of the appropriate size.
- Nonce size: Aes128Gcm/256Gcm use 12 bytes (`[u8; 12]`), ChaCha20Poly1305 uses 12 bytes (`[u8; 12]`).
- The nonce must be unique for each encryption under the same key; the library does not manage nonces—this is the caller’s responsibility.

---

## 6. Asymmetric Cryptography

### 6.1 `PrivateKey` and `PublicKey` Traits

```
pub trait PrivateKey: Sized + Dispose {
    type PublicKeyType: PublicKey;
    fn public_key(&self) -> Self::PublicKeyType;
    fn sign(&self, data: &[u8]) -> Vec<u8>;   // returns signature
    fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError>;  // RSA only, or ECIES
}

pub trait PublicKey: Sized + Copy {
    fn verify(&self, data: &[u8], signature: &[u8]) -> bool;
    fn encrypt(&self, plaintext: &[u8]) -> Vec<u8>;  // RSA or ECIES
}
```

### 6.2 RSA

```
pub struct RsaPrivateKey { … }     // linear
pub struct RsaPublicKey { … }      // Copy
impl RsaPrivateKey {
    pub fn generate(bits: usize) -> Result<RsaPrivateKey, CryptoError>;
}
impl PrivateKey for RsaPrivateKey { … }
impl PublicKey for RsaPublicKey { … }
```

- Supports key sizes of 2048, 3072, 4096 bits.
- Signatures use PKCS#1 v1.5 or PSS (configurable).
- Encryption uses OAEP.

### 6.3 Elliptic Curve (P‑256, P‑384, P‑521, Ed25519, X25519)

```
pub struct EcPrivateKey { … }   // generic over curve
pub struct EcPublicKey { … }    // Copy
impl EcPrivateKey {
    pub fn generate(curve: Curve) -> Result<EcPrivateKey, CryptoError>;
    pub fn from_bytes(curve: Curve, bytes: &[u8]) -> Result<EcPrivateKey, CryptoError>;
}
impl PrivateKey for EcPrivateKey { … }
impl PublicKey for EcPublicKey { … }

pub enum Curve {
    P256,
    P384,
    P521,
    Ed25519,
    X25519,
}
```

- `sign` produces a raw signature (for ECDSA) or a 64‑byte signature (Ed25519).  Verification uses the corresponding public key.
- X25519 provides Diffie‑Hellman key exchange:

```
impl EcPrivateKey {
    pub fn diffie_hellman(&self, peer_public: &EcPublicKey) -> Result<Vec<u8>, CryptoError>;
}
```

---

## 7. Key Derivation (HKDF, PBKDF2)

### 7.1 `Hkdf`

```
pub struct Hkdf<H: Hash> { … }
impl<H: Hash> Hkdf<H> {
    pub fn extract(salt: Option<&[u8]>, ikm: &[u8]) -> Vec<u8>;
    pub fn expand(prk: &[u8], info: &[u8], length: usize) -> Vec<u8>;
}
```

- Standard HKDF defined in RFC 5869.

### 7.2 `Pbkdf2`

```
pub fn pbkdf2(password: &[u8], salt: &[u8], iterations: u32, out_len: usize) -> Vec<u8>;
```

- Password‑based key derivation using HMAC‑SHA256 internally.

---

## 8. Error Handling

```
pub enum CryptoError {
    InvalidKeyLength,
    InvalidNonceLength,
    DecryptionFailed,
    SignatureInvalid,
    VerificationFailed,
    RandomGenerationFailed,
    Parameter(String),
}
```

---

## 9. Implementation Notes

- The crate uses hardware acceleration when available: AES‑NI on x86‑64, ARM Cryptography Extensions on aarch64, and SIMD for ChaCha20/Poly1305.
- Constant‑time code is used for comparisons of MACs and for modular arithmetic in RSA/ECC.  The implementation is written in Blaze with `unsafe` blocks where necessary for raw memory operations, but the public API is entirely safe.
- All secret key types implement `Dispose` to zeroise memory on drop.  They are not `Clone` and cannot be copied.
- Randomness for key generation is obtained from `SecureRandom`, which uses the system’s CSPRNG.

---

## 10. Testing

- **Hashing:** Compare computed digests with known test vectors for each algorithm.
- **HMAC:** Test against RFC 4231 test vectors.
- **AEAD:** Test encrypt/decrypt round‑trip, verify that tampered ciphertext fails authentication, check nonce uniqueness requirements (though tested via repeated usage).
- **RSA:** Generate keys, encrypt a small message, decrypt, verify.  Sign a message, verify the signature with the public key.
- **ECC:** Same for ECDSA and Ed25519.  Test X25519 key exchange by computing shared secrets from two sides and comparing.
- **HKDF/PBKDF2:** Compare outputs with known test vectors.
- **Zeroization:** After dropping a private key, attempt to read its memory (unsafe block in test) and confirm it is zeroed.

All tests must pass on all supported platforms.
