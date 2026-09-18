# Remove Session Page Heading

## Scope

Remove the session page's redundant heading band, including its title, subtitle, and single session view tab. Keep the sidebar entry, top breadcrumb, history repair panel, and both session browsers unchanged. Other routes retain their current heading behavior.

## Implementation

Exclude the sessions route from AppShell's shared page-heading rendering condition. Do not change shared CSS, session actions, backend interfaces, or user data.

## Delivery

Add a focused source regression, run TypeScript checking and the frontend build, and refresh the default Release executable for testing.
