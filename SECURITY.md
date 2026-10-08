# Security Policy

## Supported Versions

We take the security of Stellar Splash very seriously. Security updates and patches are actively maintained for the latest release:

| Component | Supported Versions | Status |
| :--- | :--- | :--- |
| `stellar-splash-frontend` | `v1.0.x` | Supported |
| `stellar-splash-contracts` | `v0.1.x` | Supported |
| `stellar-splash-engine` | `v1.0.x` | Supported |

---

## Reporting a Vulnerability

If you discover a security vulnerability within the Stellar Splash platform (including smart contracts, replay verification, financial accounting, or frontend authentication), please report it responsibly:

### 1. Do Not Open a Public Issue
Please do **NOT** open a public GitHub issue or disclose the vulnerability publicly until it has been reviewed and addressed.

### 2. Contact the Security Team
Send an encrypted report or details to:
- **Email**: `security@stellarsplash.io`
- **Subject**: `[SECURITY VULNERABILITY] <Brief Description>`

### 3. What to Include in Your Report
Please provide:
- A description of the vulnerability and its potential impact.
- Clear reproduction steps or proof-of-concept (PoC) code/scripts.
- Specific files, smart contract functions, or API endpoints affected.
- Any suggested fixes or remediation steps if available.

---

## Response Timeline

We are committed to handling all vulnerability reports in a timely and professional manner:
- **Initial Acknowledgment**: Within 24 hours.
- **Assessment & Triage**: Within 48 hours.
- **Remediation & Patch**: Timeline will depend on severity, with critical issues addressed promptly.
- **Public Disclosure**: Coordinated disclosure after patches are deployed to production and verified.

---

## Security Best Practices for Users
- Always verify you are interacting with the official smart contract ID on Stellar Testnet:
  `CACHWJEZY6JN36VFACWRQ7FP4T5EPRRRNBVLOSFHXTJHETZY36JD7QYJ`
- Never share your secret key (`S...`) with anyone.
- Verify that your wallet is connected to the official web application at:
  `https://stellar-splash.netlify.app`
