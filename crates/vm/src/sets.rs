use essential_types::Word;

use crate::{
    error::{DecodeError, EncodeError, OpSyncResult, StackError},
    Stack,
};

#[cfg(test)]
mod tests;

/// Encode a set into the stack.
pub(crate) fn encode_set<S, I>(set: S, stack: &mut Stack) -> OpSyncResult<()>
where
    I: ExactSizeIterator<Item = Word>,
    S: ExactSizeIterator<Item = I>,
{
    let mut len = set.len();
    for item in set {
        let item_len = item.len();
        len = len
            .checked_add(item_len)
            .ok_or(EncodeError::ItemLengthTooLarge(len))?;
        stack.extend(item)?;
        stack.push(
            item_len
                .try_into()
                .map_err(|_| EncodeError::ItemLengthTooLarge(item_len))?,
        )?;
    }
    stack.push(
        len.try_into()
            .map_err(|_| EncodeError::ItemLengthTooLarge(len))?,
    )?;
    Ok(())
}

/// Decode a set, starting from the top of slice.
pub(crate) fn decode_set(words: &[Word]) -> impl '_ + Iterator<Item = OpSyncResult<&[Word]>> {
    let mut ws = words;
    std::iter::from_fn(move || {
        let (len, rest) = ws.split_last()?;
        let ix = match usize::try_from(*len)
            .map_err(|_| StackError::Overflow.into())
            .and_then(|len| {
                rest.len()
                    .checked_sub(len)
                    .ok_or_else(|| DecodeError::Set(words.to_vec()).into())
            }) {
            Ok(ix) => ix,
            Err(e) => return Some(Err(e)),
        };
        let (rest, key) = rest.split_at(ix);
        ws = rest;
        Some(Ok(key))
    })
}
