import localFont from "next/font/local";

// Self-hosted so development and production builds work offline.
export const manrope = localFont({
  display: "swap",
  src: [
    {
      path: "../app/fonts/Manrope-Variable.ttf",
      style: "normal",
      weight: "200 800",
    },
  ],
  variable: "--font-manrope",
});

// Display headings. The slanted styles are deliberately omitted: they are
// optical variants of the upright cut, not true italics.
export const subjectivity = localFont({
  display: "swap",
  src: [
    {
      path: "../app/fonts/Subjectivity-Thin.woff2",
      style: "normal",
      weight: "100",
    },
    {
      path: "../app/fonts/Subjectivity-Light.woff2",
      style: "normal",
      weight: "300",
    },
    {
      path: "../app/fonts/Subjectivity-Regular.woff2",
      style: "normal",
      weight: "400",
    },
    {
      path: "../app/fonts/Subjectivity-Medium.woff2",
      style: "normal",
      weight: "500",
    },
    {
      path: "../app/fonts/Subjectivity-Bold.woff2",
      style: "normal",
      weight: "700",
    },
    {
      path: "../app/fonts/Subjectivity-ExtraBold.woff2",
      style: "normal",
      weight: "800",
    },
    {
      path: "../app/fonts/Subjectivity-Black.woff2",
      style: "normal",
      weight: "900",
    },
    {
      path: "../app/fonts/Subjectivity-Super.woff2",
      style: "normal",
      weight: "950",
    },
  ],
  variable: "--font-subjectivity",
});
