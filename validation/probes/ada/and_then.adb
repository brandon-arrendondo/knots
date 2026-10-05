-- probe: a sequence of like short-circuit operators
-- source: McCabe extended to conditions (Myers 1977): each and then is a decision;
--         Cognitive 1.7: one +1 for a sequence of like operators
-- expect F mccabe=4 cognitive=2
function F (A, B, C : Boolean) return Integer is
begin
   if A and then B and then C then
      return 1;
   end if;
   return 0;
end F;
