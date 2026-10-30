import { redirect } from "next/navigation";

export { privateMetadata as metadata } from "../../lib/seo";

const GetStartedPage = () => {
  redirect("/dashboard");
};

export default GetStartedPage;
