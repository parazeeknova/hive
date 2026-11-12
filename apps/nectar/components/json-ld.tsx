/** Server-rendered structured data. `<` is escaped so data cannot close the tag. */
export const JsonLd = ({ data }: { data: unknown }) => (
  <script
    type="application/ld+json"
    // oxlint-disable-next-line react/no-danger -- Server-owned JSON-LD escapes HTML delimiters.
    dangerouslySetInnerHTML={{
      __html: JSON.stringify(data).replaceAll("<", "\\u003C"),
    }}
  />
);
