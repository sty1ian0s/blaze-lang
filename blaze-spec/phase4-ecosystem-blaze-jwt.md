# Phase 4 – Ecosystem Crate: `blaze‑jwt`

> **Goal:** Provide a pure, zero‑copy JSON Web Token (JWT) implementation built on `blaze‑crypto`, `blaze‑serde`, and `blaze‑json`.  The crate supports signing and verifying tokens with HMAC (HS256, HS384, HS512), RSA (RS256, RS384, RS512), and ECDSA (ES256, ES384, ES512) algorithms.  All operations are pure; the crate does not perform I/O.  Token validation (expiration, audience, issuer, etc.) is built‑in and extensible via a simple callback.

---

## 1. Core Types

### 1.1 `Token`

```
pub struct Token {
    pub header: Header,
    pub claims: Claims,
    pub signature: Vec<u8>,
}
```

- Represents a complete JWT.  Linear (move semantics); signing consumes the token and produces a `SignedToken`.

### 1.2 `Header`

```
pub struct Header {
    pub algorithm: Algorithm,
    pub type_: Option<Text>,   // "JWT"
    pub kid: Option<Text>,      // key ID
    pub extra: Map<Text, JsonValue>,
}
```

- `Algorithm` enum: `HS256`, `HS384`, `HS512`, `RS256`, `RS384`, `RS512`, `ES256`, `ES384`, `ES512`, `None`.
- `extra` stores any additional header parameters (e.g., `cty`, `x5c`).

### 1.3 `Claims`

```
pub struct Claims {
    pub issuer: Option<Text>,        // iss
    pub subject: Option<Text>,       // sub
    pub audience: Option<Text>,      // aud (may be a single string or set)
    pub expiration: Option<u64>,     // exp (seconds since epoch)
    pub not_before: Option<u64>,     // nbf
    pub issued_at: Option<u64>,      // iat
    pub jwt_id: Option<Text>,        // jti
    pub extra: Map<Text, JsonValue>, // custom claims
}
```

- All standard registered claim names are mapped to typed fields; custom claims go into `extra`.

### 1.4 `SignedToken`

```
pub struct SignedToken {
    pub encoded: Text,   // the complete base64‑encoded token
}
```

- Created by signing a `Token`.  Can be verified and decoded back to `Token`.

---

## 2. Signing

### 2.1 `SigningKey`

```
pub enum SigningKey {
    Hmac(Vec<u8>),
    Rsa(RsaPrivateKey),
    Ec(EcPrivateKey),
}

impl SigningKey {
    pub fn from_secret(secret: &[u8]) -> SigningKey;
    pub fn from_rsa_key(key: RsaPrivateKey) -> SigningKey;
    pub fn from_ec_key(key: EcPrivateKey) -> SigningKey;
}
```

### 2.2 `sign`

```
pub fn sign(token: &Token, key: &SigningKey) -> Result<SignedToken, JwtError>;
```

- Serializes the header and claims to JSON, base64‑encodes them, computes the signature using the specified algorithm, and assembles the final token string.

---

## 3. Verification

### 3.1 `VerificationKey`

```
pub enum VerificationKey {
    Hmac(Vec<u8>),
    Rsa(RsaPublicKey),
    Ec(EcPublicKey),
    None,
}
```

### 3.2 `verify`

```
pub fn decode_and_verify(token: &SignedToken, key: &VerificationKey) -> Result<Token, JwtError>;
```

- Decodes the token, verifies the signature, and returns the header and claims.
- `decode_without_verification` is also provided (returns `Token` without checking signature).

### 3.3 Token Validation

A `TokenValidator` can be attached to the verification process to check standard claims:

```
pub struct TokenValidator {
    pub validate_exp: bool,
    pub validate_nbf: bool,
    pub validate_iss: Option<Text>,
    pub validate_aud: Option<Text>,
    pub leeway: u64,   // seconds of tolerance for exp/nbf
    pub custom_validator: Option<fn(&Claims) -> bool>,
}
```

- `verify_and_validate` performs both signature verification and claims validation.

---

## 4. Error Handling

```
pub enum JwtError {
    InvalidToken,
    InvalidHeader,
    InvalidSignature,
    InvalidKey,
    TokenExpired,
    TokenNotYetValid,
    InvalidIssuer,
    InvalidAudience,
    CustomValidationFailed,
    Serialization(Text),
    Base64(Text),
    Crypto(CryptoError),
}
```

---

## 5. Integration with `blaze‑serde`

Both `Header` and `Claims` implement `Serialize` and `Deserialize`.  The `Token` can be serialized/deserialized from JSON for debugging, but the signed token is a compact string format, not JSON.

---

## 6. Testing

- **Round‑trip:** Sign a token with each algorithm, verify, and compare claims.
- **Expiration:** Create a token with a past `exp`, verify that validation fails with `TokenExpired`.
- **Issuer/Audience:** Set validators and ensure mismatches are caught.
- **None algorithm:** A token signed with `None` must be rejected unless explicitly allowed.
- **Invalid signature:** Tamper with a token, verify returns `InvalidSignature`.

All tests must pass on all platforms.
