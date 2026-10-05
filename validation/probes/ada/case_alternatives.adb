-- probe: case alternatives (Ada's switch); when others is the default
-- source: Cognitive 1.7: under Cyclomatic Complexity each case causes an increment and the default does not
--         (getWords: three cases and a default, CC 4); a case statement is +1 to Cognitive
-- expect P mccabe=4 cognitive=1
procedure P (X : Integer; Y : out Integer) is
begin
   case X is
      when 1 =>
         Y := 10;
      when 2 =>
         Y := 20;
      when 3 =>
         Y := 30;
      when others =>
         Y := 0;
   end case;
end P;
