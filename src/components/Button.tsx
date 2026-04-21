type Props = {
  children: React.ReactNode;
  type: "button" | "submit" | "reset";
  onClick?: () => void;
};

function Button({ children, type = "button", onClick }: Props) {
  return (
    <button
      className="bg-[#FD6000] text-l text-white rounded-md p-3"
      type={type}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

export default Button;
