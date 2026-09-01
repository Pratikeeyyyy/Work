import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ToastProvider } from "../components/Toast";
import Login from "../pages/Login";
import { api, auth } from "../api";

vi.mock("../api", () => ({
  api: {
    login: vi.fn().mockResolvedValue({ token: "t", username: "alice" }),
    register: vi.fn().mockResolvedValue({ token: "t", username: "alice" }),
  },
  auth: {
    getToken: vi.fn(),
    setToken: vi.fn(),
    clearToken: vi.fn(),
  },
}));

function renderLogin() {
  return render(
    <ToastProvider>
      <Login />
    </ToastProvider>,
  );
}

function getUsername() {
  return screen.getByLabelText(/username/i);
}
function getPassword() {
  return screen.getByLabelText(/^password/i);
}
function getConfirm() {
  return screen.getByLabelText(/confirm password/i);
}

describe("Login form validation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Object.defineProperty(window, "location", {
      writable: true,
      value: { reload: vi.fn(), href: "http://localhost/" },
    });
  });

  it("shows required errors on an empty submit", async () => {
    renderLogin();
    await userEvent.click(screen.getByRole("button", { name: /log in/i }));
    expect(await screen.findByText("Username is required")).toBeInTheDocument();
    expect(screen.getByText("Password is required")).toBeInTheDocument();
    expect(api.login).not.toHaveBeenCalled();
  });

  it("rejects a username that is too short or has invalid characters", async () => {
    const user = userEvent.setup();
    renderLogin();
    await user.type(getUsername(), "ab");
    await user.click(screen.getByRole("button", { name: /log in/i }));
    expect(screen.getByText("Username must be 3-64 characters")).toBeInTheDocument();

    fireEvent.change(getUsername(), { target: { value: "al!ce" } });
    await user.click(screen.getByRole("button", { name: /log in/i }));
    expect(
      screen.getByText("Usernames can only contain letters, digits, _ - ."),
    ).toBeInTheDocument();
    expect(api.login).not.toHaveBeenCalled();
  });

  it("clears a field error once the user fixes the value", async () => {
    const user = userEvent.setup();
    renderLogin();
    await user.type(getUsername(), "ab");
    fireEvent.blur(getUsername());
    expect(await screen.findByText("Username must be 3-64 characters")).toBeInTheDocument();
    await user.clear(getUsername());
    await user.type(getUsername(), "alice");
    fireEvent.blur(getUsername());
    expect(
      screen.queryByText("Username must be 3-64 characters"),
    ).not.toBeInTheDocument();
  });

  it("requires matching passwords on register", async () => {
    const user = userEvent.setup();
    renderLogin();
    await user.click(screen.getByRole("button", { name: /create an account/i }));
    await user.type(getUsername(), "alice");
    await user.type(getPassword(), "secret123");
    await user.type(getConfirm(), "secret456");
    await user.click(screen.getByRole("button", { name: /create account/i }));
    expect(await screen.findByText("Passwords do not match")).toBeInTheDocument();
    expect(api.register).not.toHaveBeenCalled();
  });

  it("enforces a minimum password length on register", async () => {
    const user = userEvent.setup();
    renderLogin();
    await user.click(screen.getByRole("button", { name: /create an account/i }));
    await user.type(getUsername(), "alice");
    await user.type(getPassword(), "short");
    await user.type(getConfirm(), "short");
    await user.click(screen.getByRole("button", { name: /create account/i }));
    expect(
      await screen.findByText("Password must be at least 8 characters"),
    ).toBeInTheDocument();
    expect(api.register).not.toHaveBeenCalled();
  });

  it("submits valid login credentials and stores the token", async () => {
    const user = userEvent.setup();
    renderLogin();
    await user.type(getUsername(), "alice");
    await user.type(getPassword(), "secret123");
    await user.click(screen.getByRole("button", { name: /log in/i }));
    expect(api.login).toHaveBeenCalledWith("alice", "secret123");
    expect(auth.setToken).toHaveBeenCalledWith("t");
    expect(window.location.reload).toHaveBeenCalled();
  });

  it("submits valid new-account credentials on register", async () => {
    const user = userEvent.setup();
    renderLogin();
    await user.click(screen.getByRole("button", { name: /create an account/i }));
    await user.type(getUsername(), "alice");
    await user.type(getPassword(), "secret123");
    await user.type(getConfirm(), "secret123");
    await user.click(screen.getByRole("button", { name: /create account/i }));
    expect(api.register).toHaveBeenCalledWith("alice", "secret123");
    expect(auth.setToken).toHaveBeenCalledWith("t");
  });
});