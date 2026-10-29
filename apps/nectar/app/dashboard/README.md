# Workspace design and integration

The dashboard shares the homepage’s Subjectivity/Manrope typography, rounded controls, dark primary buttons, and restrained meadow colors. Its layout uses all available width rather than centering a marketing hero in the workspace.

## Changes from the audit

- One grouped navigation replaces duplicate sidebar and lifecycle navigation.
- Discover opens directly into search/results and a full-height capability inspector. Its visible heading and Create capability button have been removed.
- Website Compose and Publish flows are removed. Agents create capabilities through their own connection. Legacy `tools`, `compose`, and `publish` view links resolve to Account settings.
- My capabilities lives in Account settings, alongside avatar, name, username, agent balance, and a collapsible session/agent connection section. Its rows and inspector follow Discover's visual hierarchy.
- Account settings uses a single-line heading and three tabs: Profile (including session/agent connection), Agent balance (including the dismissible starter-credit prompt and funding actions), and Capabilities (list and settings inspector). Tabs support arrow keys, Home/End, and roving keyboard focus. Hidden panels stay mounted so tab changes preserve unsaved edits, search, and capability selection.
- All tabs share one full-width settings frame, matching main/secondary columns, 20px desktop padding, 40px inputs, and consistently sized actions. Profile's agent connection and Balance's payment status occupy the secondary column. Account columns stack below 900px; the preview/storage note lives in the frame footer.
- The top bar omits the home link and preview indicator; the sidebar omits the workspace introduction. Its Agent balance shortcut reads exactly the same funding state and health color as the balance tab and opens it directly. Account tab selection is reflected in the `tab` query parameter for reloads and direct links.
- Selecting a capability exposes free/paid pricing, an exact USDC per-call price, and marketplace visibility. Disabled preview capabilities remain in Account settings and disappear from preview Discover results. The previous paid price is retained when switching to free.
- Account settings are explicitly a local preview: profile and capability preferences are saved per wallet (or preview) in browser storage. There are no profile-update, capability-update, or complete creator-inventory endpoints in the current contract. Connected accounts show only owned capabilities in the loaded registry, which is limited to 50 active results.
- New previews start with $50 USD of starter credit. A dismissible welcome prompt explains the new-account credit; dismissal is persisted without changing the balance. The balance-health bar is red at $10 or less, amber up to $25, and green above $25, capped visually at the $50 starter-credit reference.
- Top up, Send, and Receive open reviewable payment dialogs. Preview-only demo top-ups update local funds without charging a payment method. Connected accounts show balance unavailable until a real funding source is supplied; session authorization is never treated as wallet funds.
- Search, loaded-result filters, sorting, and capability rows stay together.
- The inspector shows an author avatar placeholder and creator wallet above the title, followed by price and seven disclosure sections: Overview, Input / Output Schema, Sample Response, Dependencies, Pricing, Limitations, and Provider. It uses a flat background with no artifact logos.
- Test with sample input opens an explicit placeholder; Invoke in agent opens the repository/commit quote form in a dialog. The registry has no author username, profile photo, or sample execution endpoint yet.
- Below 1,100px, selecting a capability opens its inspector above the results; below 760px, navigation becomes a drawer with keyboard focus management.
- Session authorization replaces fictional monthly spending. Reserved budget and spent budget remain distinct from wallet funds.
- Runtime labels follow the contract. Verification, speed estimates, arbitrary agents, and unsupported management actions are not represented as live data.

## Integration

Set `NEXT_PUBLIC_HIVEWAY_ORIGIN` to the API origin (default: `http://127.0.0.1:3001`). Configure the backend to accept Nectar’s exact origin. Bearer credentials remain in memory and are cleared on reload/disconnect.

The disconnected catalog contains three clearly labeled examples. Connecting loads the protected registry. Discovery uses the server’s `q` search; runtime filters and sorting operate only on the returned records (at most 50).

The workspace connects wallet challenges, credential status, discovery, quotes, activity/receipt reconciliation, and creator sales to the documented REST routes. It requests no invented endpoints. All monetary parsing and formatting uses atomic strings and bigint arithmetic.

## Replaceable component boundaries

- `PaymentAuthorizationPlaceholder` in `workspace-pages.tsx` is the insertion point for a future native x402 wallet payment component. The current quote handoff copies only the invocation URL, ID, and returned payment requirements; it does not submit a payment or include the bearer credential.
- The Session view’s client setup placeholder can be replaced by guided MCP configuration. The current view shows the gateway URL and header format.
- `AccountPaymentDialog` in `account-payments.tsx` is the funding integration boundary. Dodo checkout and live transfers are clearly unavailable. A future backend must create checkout sessions and credit funds only after verified, idempotent payment webhooks; a checkout return URL cannot credit the balance. Dodo merchant payouts must not be treated as arbitrary peer-to-peer transfers. Profile persistence, creator inventory including disabled listings, and marketplace mutations also need authenticated server endpoints.

Browser and live backend integration testing are intentionally left to the user.
