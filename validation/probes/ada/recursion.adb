-- probe: direct recursion
-- source: Cognitive 1.7 section "Recursion": +1 for each method in a recursion cycle; if +1, else +1
-- expect Fact mccabe=2 cognitive=3
function Fact (N : Natural) return Natural is
begin
   if N <= 1 then
      return 1;
   else
      return N * Fact (N - 1);
   end if;
end Fact;
