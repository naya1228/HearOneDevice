type Props = {
  children: React.ReactNode;
  onClick?: () => void;
};

function Button({ children, onClick }: Props) {
  return (
    <button
      className="bg-[#FD6000] text-xl font-medium text-white rounded-md p-5"
      onClick={onClick}
    >
      {children}
    </button>
  );
}

export default Button;
