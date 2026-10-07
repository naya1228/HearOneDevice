type Props = {
  children: React.ReactNode;
  type: "button" | "submit" | "reset";
  variant?: "primary" | "secondary";
  disabled?: boolean;
  onClick?: () => void;
};

const colors = {
  primary: "bg-[#FD6000] hover:bg-[#E55600]",
  secondary: "bg-[#3A3A39] hover:bg-[#454544]",
};

function Button({ children, type = "button", variant = "primary", disabled, onClick }: Props) {
  return (
    <button
      className={`${colors[variant]} w-full text-white font-medium rounded-lg p-3 disabled:opacity-60`}
      type={type}
      disabled={disabled}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

export default Button;
