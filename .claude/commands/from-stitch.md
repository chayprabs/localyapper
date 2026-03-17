Convert a design from Google Stitch to a React component. The screen/component to convert: $ARGUMENTS

1. Use the Stitch MCP to fetch the design data for the specified screen
2. Map design elements to shadcn/ui components (Button, Input, Card, Dialog, Select, etc.)
3. Use Tailwind utility classes matching our DESIGN_SYSTEM.md tokens exactly
4. Create a TypeScript functional component with proper props interface
5. Named export, place in the appropriate src/components/ subdirectory
6. NO custom CSS files — Tailwind only
7. Ensure all colors, fonts, spacing match DESIGN_SYSTEM.md

Show me the component code and ask for approval before writing the file.
