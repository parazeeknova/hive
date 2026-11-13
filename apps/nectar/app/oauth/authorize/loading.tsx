import { LoadingState } from "../../../components/spinner";

const AuthorizeLoading = () => (
  <LoadingState
    screen
    label="Checking the app’s request…"
    className="bg-[#fffefb]"
  />
);

export default AuthorizeLoading;
