# Remove Redundant Session and Supplier Page Headings

## Scope

Remove the session, supplier, and theme pages' redundant heading band, including the page title and subtitle. Keep the sidebar entries, top breadcrumb, session controls, supplier routing controls, supplier cards, and theme content unchanged. Other routes retain their current heading behavior.

## Implementation

Exclude the sessions, supplier, and themes routes from AppShell's shared page-heading rendering condition. Do not change shared CSS, session/supplier/theme actions, backend interfaces, or user data.

## Delivery

Add a focused source regression, run TypeScript checking and the frontend build, and refresh the default Release executable for testing.
